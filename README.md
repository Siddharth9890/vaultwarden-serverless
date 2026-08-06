

# Vaultwarden Serverless on AWS Lambda

## Overview

This repository contains a custom AWS Lambda build of Vaultwarden with the following modifications:

* AWS Lambda runtime support
* Persistent JWT signing keys (prevents random logouts)
* PostgreSQL support (Neon compatible)
* Amazon Linux 2023 build environment
* Updated OpenSSL/libpq runtime
* Automatic runtime dependency packaging

---

# Features

* ✅ Runs on AWS Lambda
* ✅ PostgreSQL (Neon) support
* ✅ SQLite support
* ✅ Persistent JWT keys
* ✅ Automatic packaging of required shared libraries
* ✅ Amazon Linux 2023 compatible
* ✅ Works with modern PostgreSQL TLS

---

# Repository Layout

```
build/
│
├── Dockerfile
├── build.sh
├── sample.rs       
├── vaultwarden/
│     ├── Cargo.toml
│     ├── bootstrap.zip <- Working prebuilt binary
│     ├── src/
│     │      auth.rs
│     │      main.rs
│     └── ...
```

---

# If You Only Want To Deploy

A prebuilt working

```
bootstrap.zip
```

already exists.

If it works for your deployment, **you do not need to rebuild Vaultwarden**.

Simply upload it to Lambda and configure your environment variables.

---

# Building From Source

## 1. Clone

```bash
git clone <repo>

cd build
```

Clone Vaultwarden

```bash
git clone https://github.com/dani-garcia/vaultwarden vaultwarden
```

---

## 2. Apply Custom Changes

Reapply the following project changes:

### auth.rs

Replace JWT logic using

```
sample.rs
```

This fixes Lambda container restarts causing random user logout.

---

### main.rs

Reapply Lambda runtime changes.

The standard Vaultwarden main function will not run correctly on Lambda.

Check

```
src/main.rs
```

from this repository and copy the Lambda-specific logic after updating Vaultwarden.

---

### Cargo.toml

Reapply Lambda dependencies.

When updating Vaultwarden always compare:

```
Cargo.toml
```

with the version in this repository.

Required Lambda crates have been added.

---

# Docker Build

## Apple Silicon (M1/M2/M3)

Always use Docker Buildx.

Normal Docker builds create ARM images which generate ARM binaries.

AWS Lambda x86_64 requires an x86 binary.

Build using:

```bash
docker buildx build \
    --platform linux/amd64 \
    -t lambda-builder \
    --load .
```

Verify:

```bash
docker image inspect lambda-builder
```

Should show

```
Architecture: amd64
```

---



# Build Bootstrap

Run

```bash
docker run --rm \
    --mount type=bind,source="$(pwd)/vaultwarden-serverless",target=/build \
    lambda-builder
```

Generated output

```
bootstrap.zip
```

---

# Amazon Linux 2023 Migration

Earlier versions used Amazon Linux 2.

This no longer works reliably with modern PostgreSQL providers because:

* AL2 ships PostgreSQL 9.2 client libraries
* libpq links against OpenSSL 1.0
* Modern PostgreSQL providers (such as Neon) require newer TLS/OpenSSL support

Symptoms include

```
could not create SSL context
SSL error code 168296468
```

The build system has therefore been migrated to

```
amazonlinux:2023
```

which provides

* OpenSSL 3
* libpq 18
* Updated TLS stack
* Also recommeded by Amazon as AL2 will be deparacted

---

# Shared Library Packaging

The build process automatically packages all required runtime libraries.

Including

* libpq
* libssl
* libcrypto
* kerberos libraries
* ldap libraries
* sasl libraries
* CA certificates

These are discovered automatically using

```
ldd
```

rather than maintaining a manual copy list.

---

# JWT Fix

Vaultwarden normally generates a new RSA key every process start.

Lambda containers are short lived.

Result:

Users are randomly logged out.

Solution:

Generate one RSA key.

```bash
openssl genpkey \
    -algorithm RSA \
    -pkcs8 \
    -out rsa_key.pem
```

Store entire key inside

```
VAULTWARDEN_RSA_KEY
```

environment variable.

Now every Lambda container uses the same signing key.

---

# Required Environment Variables

```
DATABASE_URL=...

DOMAIN=https://...

ROCKET_ADDRESS=0.0.0.0

ROCKET_PORT=8000

VAULTWARDEN_RSA_KEY=<private key>

RUST_LOG=info
```

---

# Lambda Configuration

Runtime

```
provided.al2023
```

Handler

```
bootstrap
```

Architecture

```
x86_64
```

Memory

```
512 MB+
```

Timeout

```
30-60 seconds
```

---

# Updating Vaultwarden

Whenever updating to a newer release:

1. Pull latest Vaultwarden.
2. Compare Cargo.toml.
3. Reapply main.rs Lambda changes.
4. Reapply auth.rs JWT patch.
5. Build using Docker Buildx.
6. Upload new bootstrap.zip.

Do **not** overwrite your custom changes.

---

# Troubleshooting

## SSL Error

```
could not create SSL context
```

Usually means

* built on AL2
* old libpq
* wrong OpenSSL

Use Amazon Linux 2023.

---

## ARM Binary

Check

```bash
file bootstrap
```

Correct output

```
ELF 64-bit LSB executable, x86-64
```

If it says

```
ARM aarch64
```

Rebuild using

```
docker buildx --platform linux/amd64
```

---

## Missing Shared Libraries

Example

```
libldap_r-2.4.so.2
```

The automatic packaging step was skipped.

Run the build again.

---

## Random Logouts

Verify

```
VAULTWARDEN_RSA_KEY
```

is configured.

Without it JWT signing keys change every cold start.

---

# Security

* Never commit your RSA private key.
* Disable debug logging in production.
* Use TLS-enabled PostgreSQL.

---

# Notes

* The repository already contains a **known working `bootstrap.zip`**. If rebuilding fails during future updates, you can continue using that artifact while investigating the build issue.
* This project is based on upstream Vaultwarden. After every upstream update, review and reapply the custom Lambda integration (`src/main.rs`), JWT changes (`src/auth.rs`), and any required `Cargo.toml` additions before producing a new release.
* The build process targets **Amazon Linux 2023** and **AWS Lambda x86_64**. Avoid switching back to Amazon Linux 2, as its older PostgreSQL/OpenSSL stack is incompatible with many modern managed PostgreSQL services.
