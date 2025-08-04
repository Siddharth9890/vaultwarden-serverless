# Vaultwarden Serverless on AWS Lambda

This guide walks you through deploying Vaultwarden on AWS Lambda with persistent JWT authentication.

## 🚀 Quick Start

### Prerequisites
- Docker installed
- AWS CLI configured
- Access to AWS Lambda console

### Step 1: Clone and Setup Repository
```bash
# Clone your main repository
git clone <your-repo-url>
cd <your-repo-name>

# Navigate to build directory
cd build/vaultwarden-serverless

# Clone the official Vaultwarden repository
git clone https://github.com/dani-garcia/vaultwarden.git .

# Go back to build directory
cd ..
```

### Step 2: Build Docker Image
```bash
# Build the Lambda builder image
docker build . -t lambda-builder
```

### Step 3: Apply Custom JWT Fix
```bash
# Copy libpq.so.5 to vaultwarden-serverless directory first
cp libpq.so.5 ./vaultwarden-serverless/

# IMPORTANT: Apply the custom JWT authentication fix
# Replace the JWT functions in src/auth.rs with the code from sample.rs
# This fixes the random logout issue in AWS Lambda
cp sample.rs ./vaultwarden-serverless/src/auth_backup.rs  # Backup original
# Then manually apply the changes from sample.rs to src/auth.rs
```

### Step 4: Build the Lambda Package
```bash
# Run the Docker build process
docker run --rm --mount type=bind,source="./vaultwarden-serverless/",target=/build lambda-builder
```

### Step 5: Deploy to AWS Lambda
1. The build process will generate `bootstrap.zip`
2. Upload `bootstrap.zip` to your AWS Lambda function
3. Update the function code in AWS Lambda UI

### Step 6: Configure JWT Authentication

#### Generate RSA Key
```bash
# Generate a persistent RSA key (PKCS#8 format)
openssl genpkey -algorithm RSA -pkcs8 -out rsa_key.pem

# View the key content
cat rsa_key.pem
```

#### Set Environment Variable
1. Go to AWS Lambda Console
2. Navigate to your Vaultwarden function
3. Configuration → Environment variables
4. Add new environment variable:
   - **Key:** `VAULTWARDEN_RSA_KEY`
   - **Value:** [Paste the entire RSA key content including BEGIN/END lines]

**Example key format:**
```
-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC4f6c1kO+cUbHb
lPk8JRNcXFBvZxwcF+q7+GKsqD+ZGm4FQb0h9Q3YjKzD7wZ8qF9A5tKgT1vW4xYK
...
-----END PRIVATE KEY-----
```

### Step 7: Configure Other Environment Variables
Set these additional environment variables in AWS Lambda:

```bash
# Database configuration
DATABASE_URL=postgresql://username:password@host:port/database

# Domain settings
DOMAIN=https://your-vaultwarden-domain.com
ROCKET_ADDRESS=0.0.0.0
ROCKET_PORT=8000

# Optional: Enable debug logging (remove in production)
RUST_LOG=debug
```
### Step 8: Check prev changes
Check prev git history about file changes for example main.rs it has main lambda code 

## 🔧 Troubleshooting

### ⚠️ CRITICAL: Always Apply Custom JWT Code
**If you skip applying the `sample.rs` changes to `src/auth.rs`, you WILL experience random logouts!**

```bash
# Verify the custom code is applied:
grep -n "VAULTWARDEN_RSA_KEY" ./vaultwarden-serverless/src/auth.rs

# Should return a line showing the environment variable check
# If not found, you need to apply the sample.rs changes
```

### JWT Authentication Fix (sample.rs)

The `sample.rs` file contains the modified JWT authentication code that fixes random logouts in AWS Lambda. You **must apply these changes** to `src/auth.rs` after cloning Vaultwarden but before building.

#### Key Changes Made:
1. **`initialize_keys()` function**: Modified to use persistent RSA keys
2. **`get_or_generate_rsa_key()` function**: New function that checks `VAULTWARDEN_RSA_KEY` environment variable first
3. **Enhanced error logging**: Better debugging for JWT issues
4. **Serverless compatibility**: Handles read-only filesystem in Lambda

#### How to Apply:
```bash
# After cloning Vaultwarden, before building:
cd build/vaultwarden-serverless

# Backup the original auth.rs
cp src/auth.rs src/auth.rs.backup

# Apply the changes from sample.rs to src/auth.rs
# You need to manually replace the following functions in src/auth.rs:
# - initialize_keys()
# - Add the new get_or_generate_rsa_key() function
# - Optionally enhance encode_jwt() and decode_jwt() for better logging
```

#### Why This Fix is Needed:
- **Problem**: Original Vaultwarden generates new RSA keys on each Lambda container start
- **Effect**: Users get randomly logged out when Lambda containers restart
- **Solution**: Use persistent RSA key from environment variable
- **Result**: Users stay logged in across Lambda container lifecycles

#### Files Modified:
- `src/auth.rs` - Core JWT authentication functions
- The changes are **backward compatible** with standard Vaultwarden deployments

### Common Issues

#### Random Logouts Fixed! ✅
The persistent RSA key in `VAULTWARDEN_RSA_KEY` solves the random logout issue caused by Lambda container recycling.

#### JWT Authentication Errors
If you see "Error decoding JWT" or "Invalid claim" errors:

1. **Check RSA key format:** Must be PKCS#8 format (`-----BEGIN PRIVATE KEY-----`)
2. **Verify environment variable:** Ensure the entire key is copied including newlines
3. **Check logs:** Look for JWT debug messages in CloudWatch

#### Build Issues
- Ensure `libpq.so.5` is copied to `vaultwarden-serverless/` before running Docker
- Make sure Docker has sufficient memory allocated
- Check that all paths are correct relative to the build directory

### Debug Logging
To see detailed JWT authentication logs, set:
```bash
RUST_LOG=debug
```

Look for these log patterns in CloudWatch:
```
🔐 [DEBUG] JWT keys initialized successfully with fingerprint: a1b2c3d4
🎫 [DEBUG] JWT token generated successfully!
🔓 [DEBUG] JWT successfully decoded!
```

### Key Rotation
To rotate the RSA key:
1. Generate a new key using the command above
2. Update the `VAULTWARDEN_RSA_KEY` environment variable
3. **Note:** This will log out all existing users

## 📁 Project Structure
```
build/
├── Dockerfile                 # Lambda builder image
├── libpq.so.5                # PostgreSQL library
├── sample.rs                  # 🔑 Custom JWT authentication code (IMPORTANT!)
├── vaultwarden-serverless/    # Vaultwarden source code
│   ├── src/
│   │   ├── auth.rs           # ⚠️  Apply sample.rs changes here
│   │   └── ...
│   └── vaultwarden/          # Cloned repository
└── bootstrap.zip             # Generated Lambda deployment package
```

## ⚠️ Important Notes

### Custom Code Maintenance
- **Always apply `sample.rs` changes** after updating Vaultwarden source code
- The custom JWT code is **essential** for preventing random logouts in Lambda
- Keep `sample.rs` as a reference for future Vaultwarden updates
- Changes are minimal and focused only on RSA key persistence

## 🔒 Security Notes

1. **RSA Key Security:** Keep your RSA private key secure. Never commit it to version control.
2. **Environment Variables:** Use AWS Secrets Manager for production environments instead of plain environment variables.
3. **Debug Logging:** Disable debug logging (`RUST_LOG=info`) in production as it logs sensitive tokens.
4. **Database:** Ensure your database connection is encrypted and uses strong authentication.

## 🚀 Production Deployment

For production environments:

1. **Use AWS Secrets Manager** for sensitive configuration:
```bash
# Store RSA key in Secrets Manager
aws secretsmanager create-secret --name "vaultwarden/rsa-key" --secret-string "$(cat rsa_key.pem)"
```

2. **Configure proper Lambda settings:**
   - Memory: 1024MB or higher
   - Timeout: 30 seconds
   - Reserved concurrency: Based on your user count

3. **Set up monitoring:**
   - CloudWatch alarms for errors
   - Lambda insights for performance monitoring

4. **Database optimization:**
   - Use RDS with proper instance sizing
   - Enable connection pooling
   - Set up read replicas if needed

## 📞 Support

If you encounter issues:

1. Check CloudWatch logs for detailed error messages
2. Verify all environment variables are set correctly
3. Ensure the RSA key format is correct (PKCS#8)
4. Test with a fresh Lambda deployment

## 🎯 Key Benefits

- ✅ **No more random logouts** - Persistent JWT keys across Lambda container lifecycles
- ✅ **Serverless scaling** - Automatic scaling based on demand  
- ✅ **Cost effective** - Pay only for actual usage
- ✅ **Easy deployment** - Single ZIP file deployment
- ✅ **Detailed logging** - Comprehensive debug information

---

**Last Updated:** August 2025  
**Vaultwarden Version:** Latest compatible build