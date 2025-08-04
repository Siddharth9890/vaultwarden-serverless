// sample.rs - Custom JWT Authentication Code for Vaultwarden Serverless
// 
// This file contains the modified JWT authentication code that fixes the random logout issue
// in AWS Lambda by using a persistent RSA key from environment variables.
// 
// IMPORTANT: These changes need to be applied to src/auth.rs in the Vaultwarden source code
// after cloning the repository but before building.

use chrono::{TimeDelta, Utc};
use num_traits::FromPrimitive;
use once_cell::sync::{Lazy, OnceCell};

use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header};
use openssl::rsa::Rsa;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::{error::Error, CONFIG};

const JWT_ALGORITHM: Algorithm = Algorithm::RS256;

pub static DEFAULT_VALIDITY: Lazy<TimeDelta> = Lazy::new(|| TimeDelta::try_hours(2).unwrap());
static JWT_HEADER: Lazy<Header> = Lazy::new(|| Header::new(JWT_ALGORITHM));

pub static JWT_LOGIN_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|login", CONFIG.domain_origin()));
static JWT_INVITE_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|invite", CONFIG.domain_origin()));
static JWT_EMERGENCY_ACCESS_INVITE_ISSUER: Lazy<String> =
    Lazy::new(|| format!("{}|emergencyaccessinvite", CONFIG.domain_origin()));
static JWT_DELETE_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|delete", CONFIG.domain_origin()));
static JWT_VERIFYEMAIL_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|verifyemail", CONFIG.domain_origin()));
static JWT_ADMIN_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|admin", CONFIG.domain_origin()));
static JWT_SEND_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|send", CONFIG.domain_origin()));
static JWT_ORG_API_KEY_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|api.organization", CONFIG.domain_origin()));
static JWT_FILE_DOWNLOAD_ISSUER: Lazy<String> = Lazy::new(|| format!("{}|file_download", CONFIG.domain_origin()));

static PRIVATE_RSA_KEY: OnceCell<EncodingKey> = OnceCell::new();
static PUBLIC_RSA_KEY: OnceCell<DecodingKey> = OnceCell::new();

// ============================================================================
// CUSTOM CODE: Modified initialize_keys function for AWS Lambda persistence
// ============================================================================
pub fn initialize_keys() -> Result<(), crate::error::Error> {
    info!("🔐 Starting JWT key initialization for serverless environment...");
    
    let priv_key_buffer = get_or_generate_rsa_key()?;
    
    // Parse the private key
    let priv_key = Rsa::private_key_from_pem(&priv_key_buffer)?;
    let pub_key_buffer = priv_key.public_key_to_pem()?;

    // Generate a simple fingerprint for debugging
    let key_fingerprint = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        priv_key_buffer.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    info!("RSA Key fingerprint: {}", key_fingerprint);

    let enc = EncodingKey::from_rsa_pem(&priv_key_buffer)?;
    let dec: DecodingKey = DecodingKey::from_rsa_pem(&pub_key_buffer)?;
    
    if PRIVATE_RSA_KEY.set(enc).is_err() {
        err!("PRIVATE_RSA_KEY must only be initialized once")
    }
    if PUBLIC_RSA_KEY.set(dec).is_err() {
        err!("PUBLIC_RSA_KEY must only be initialized once")
    }
    
    info!("JWT keys initialized successfully with fingerprint: {}", key_fingerprint);
    Ok(())
}

// ============================================================================
// CUSTOM CODE: New function to handle persistent RSA keys for serverless
// ============================================================================
fn get_or_generate_rsa_key() -> Result<Vec<u8>, crate::error::Error> {
    use std::{fs::File, io::{Read, Write}};
    
    // 1. PRIORITY: Check environment variable (for AWS Lambda persistence)
    if let Ok(env_key) = std::env::var("VAULTWARDEN_RSA_KEY") {
        info!("Using RSA key from VAULTWARDEN_RSA_KEY environment variable");
        
        // Handle escaped newlines in environment variable
        let processed_key = env_key.replace("\\n", "\n");
        let key_bytes = processed_key.into_bytes();
        
        // Validate the key can be parsed
        match openssl::rsa::Rsa::private_key_from_pem(&key_bytes) {
            Ok(_) => {
                info!("✅ RSA key from environment validated successfully");
                return Ok(key_bytes);
            },
            Err(e) => {
                error!("❌ Failed to parse RSA key from environment: {}", e);
                error!("This will cause random logouts in serverless environments!");
                // Fall through to file/generation fallback
            }
        }
    }
    
    // 2. FALLBACK: Original file-based approach (for backward compatibility)
    info!("Environment variable not found, falling back to file-based key");
    
    let mut priv_key_buffer = Vec::with_capacity(2048);
    let priv_key = {
        let mut priv_key_file =
            File::options().create(true).truncate(false).read(true).write(true).open(CONFIG.private_rsa_key())?;

        #[allow(clippy::verbose_file_reads)]
        let bytes_read = priv_key_file.read_to_end(&mut priv_key_buffer)?;

        if bytes_read > 0 {
            info!("Using existing RSA key from file: {}", CONFIG.private_rsa_key());
            Rsa::private_key_from_pem(&priv_key_buffer[..bytes_read])?
        } else {
            // Only create the key if the file doesn't exist or is empty
            warn!("No persistent RSA key found. Generating new key - this will invalidate existing JWT tokens!");
            warn!("For serverless deployments, set VAULTWARDEN_RSA_KEY environment variable to prevent random logouts.");
            
            let rsa_key = openssl::rsa::Rsa::generate(2048)?;
            priv_key_buffer = rsa_key.private_key_to_pem()?;
            
            // Try to save to file (might fail in Lambda read-only filesystem)
            match priv_key_file.write_all(&priv_key_buffer) {
                Ok(_) => info!("Private key created and saved to file."),
                Err(_) => warn!("Could not save private key to file (read-only filesystem). Key exists only in memory."),
            }
            
            rsa_key
        }
    };

    Ok(priv_key_buffer)
}

// ============================================================================
// OPTIONAL: Enhanced JWT encoding with debug logging
// ============================================================================
pub fn encode_jwt<T: Serialize>(claims: &T) -> String {
    match jsonwebtoken::encode(&JWT_HEADER, claims, PRIVATE_RSA_KEY.wait()) {
        Ok(token) => {
            // Optional debug logging (remove in production)
            #[cfg(debug_assertions)]
            {
                debug!("JWT token generated successfully, length: {}", token.len());
                if let Ok(claims_json) = serde_json::to_string_pretty(claims) {
                    debug!("JWT claims: {}", claims_json);
                }
            }
            token
        },
        Err(e) => {
            error!("Error encoding JWT: {}", e);
            panic!("Error encoding jwt {e}")
        },
    }
}

// ============================================================================
// OPTIONAL: Enhanced JWT decoding with better error messages
// ============================================================================
fn decode_jwt<T: DeserializeOwned>(token: &str, issuer: String) -> Result<T, Error> {
    let mut validation = jsonwebtoken::Validation::new(JWT_ALGORITHM);
    validation.leeway = 30; // 30 seconds
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.set_issuer(&[issuer.clone()]);

    let token = token.replace(char::is_whitespace, "");
    match jsonwebtoken::decode(&token, PUBLIC_RSA_KEY.wait(), &validation) {
        Ok(d) => {
            debug!("JWT successfully decoded for issuer: {}", issuer);
            Ok(d.claims)
        },
        Err(err) => {
            // Enhanced error logging for debugging
            error!("JWT decode error for issuer '{}': {:?}", issuer, err.kind());
            
            match *err.kind() {
                ErrorKind::InvalidToken => {
                    error!("Token is malformed or corrupted");
                    err!("Token is invalid")
                },
                ErrorKind::InvalidIssuer => {
                    error!("Token issuer mismatch - expected: {}", issuer);
                    err!("Issuer is invalid")
                },
                ErrorKind::ExpiredSignature => {
                    error!("Token has expired");
                    err!("Token has expired")
                },
                ErrorKind::InvalidSignature => {
                    error!("Invalid signature - likely RSA key mismatch!");
                    error!("This usually indicates a serverless container restart with different keys.");
                    err!("Error decoding JWT")
                },
                _ => {
                    error!("JWT decode error: {}", err);
                    err!("Error decoding JWT")
                },
            }
        },
    }
}

// ============================================================================
// Keep all existing decode functions unchanged
// ============================================================================
pub fn decode_login(token: &str) -> Result<LoginJwtClaims, Error> {
    decode_jwt(token, JWT_LOGIN_ISSUER.to_string())
}

pub fn decode_invite(token: &str) -> Result<InviteJwtClaims, Error> {
    decode_jwt(token, JWT_INVITE_ISSUER.to_string())
}

pub fn decode_emergency_access_invite(token: &str) -> Result<EmergencyAccessInviteJwtClaims, Error> {
    decode_jwt(token, JWT_EMERGENCY_ACCESS_INVITE_ISSUER.to_string())
}

pub fn decode_delete(token: &str) -> Result<BasicJwtClaims, Error> {
    decode_jwt(token, JWT_DELETE_ISSUER.to_string())
}

pub fn decode_verify_email(token: &str) -> Result<BasicJwtClaims, Error> {
    decode_jwt(token, JWT_VERIFYEMAIL_ISSUER.to_string())
}

pub fn decode_admin(token: &str) -> Result<BasicJwtClaims, Error> {
    decode_jwt(token, JWT_ADMIN_ISSUER.to_string())
}

pub fn decode_send(token: &str) -> Result<BasicJwtClaims, Error> {
    decode_jwt(token, JWT_SEND_ISSUER.to_string())
}

pub fn decode_api_org(token: &str) -> Result<OrgApiKeyLoginJwtClaims, Error> {
    decode_jwt(token, JWT_ORG_API_KEY_ISSUER.to_string())
}

pub fn decode_file_download(token: &str) -> Result<FileDownloadClaims, Error> {
    decode_jwt(token, JWT_FILE_DOWNLOAD_ISSUER.to_string())
}

// ============================================================================
// USAGE INSTRUCTIONS:
// ============================================================================
// 
// 1. After cloning Vaultwarden repository, replace the contents of src/auth.rs 
//    with the functions above (keeping the existing struct definitions and 
//    other imports that are not shown here).
//
// 2. The key changes are:
//    - initialize_keys() now calls get_or_generate_rsa_key()
//    - get_or_generate_rsa_key() checks VAULTWARDEN_RSA_KEY environment variable first
//    - Enhanced error logging for debugging JWT issues
//
// 3. Set the VAULTWARDEN_RSA_KEY environment variable in AWS Lambda with your
//    persistent RSA private key to prevent random logouts.
//
// 4. Optional: Enable debug logging with RUST_LOG=debug to see detailed JWT info
//
// ============================================================================