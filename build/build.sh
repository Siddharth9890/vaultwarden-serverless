#!/bin/bash
set -euxo pipefail

export PATH="/root/.cargo/bin:$PATH"

cargo --version
rustc --version
cd /build

echo "Fetching latest web vault..."
curl -s -L "$(curl -s https://api.github.com/repos/dani-garcia/bw_web_builds/releases/latest \
  | jq -r '.assets[] | select(.name | endswith("tar.gz")) | .browser_download_url')" \
  | tar xz

cargo build --release \
    --features "sqlite,postgresql" \
    --target-dir target_al2023

mv target_al2023/release/vaultwarden bootstrap
echo "=== Build finished ==="
# exec /bin/bash

# echo "========== OpenSSL =========="
# openssl version -a

# echo "========== Packages =========="
# rpm -q openssl openssl-devel postgresql-libs postgresql-devel

echo "========== bootstrap =========="
ldd bootstrap

# echo "========== libpq =========="
ldd /usr/lib64/libpq.so.5

# echo "========== libpq location =========="
find /usr -name "libpq.so*"

# echo "========== mysql =========="
find /usr -name "libmysqlclient.so*"

mkdir -p lib

# Copy libpq itself
cp -L /usr/lib64/libpq.so* lib/

# Copy MySQL client if present
find /usr -name "libmysqlclient.so*" -exec cp -L {} lib/ \; || true

# Copy all non-system dependencies of libpq
ldd /usr/lib64/libpq.so.5 \
| awk '/=> \// {print $3}' \
| while read f; do
    case "$f" in
        /lib64/libc.so.*|\
        /lib64/libm.so.*|\
        /lib64/libdl.so.*|\
        /lib64/libpthread.so.*|\
        /lib64/librt.so.*|\
        /lib64/ld-linux*|\
        /lib64/libgcc_s.so.*)
            ;;
        *)
            cp -L "$f" lib/
            ;;
    esac

done

# Copy CA certificates
cp -L /etc/pki/tls/certs/ca-bundle.crt lib/ca-bundle.crt \
    || cp -L /etc/ssl/certs/ca-certificates.crt lib/ca-bundle.crt

echo "========== packaged libs =========="
ls -lh lib

zip -9 -r bootstrap.zip bootstrap web-vault lib

echo "Done."
