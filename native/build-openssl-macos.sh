#!/usr/bin/env bash
# Pinned static OpenSSL recipe; this file is the independent macOS cache input.
set -euo pipefail
if [[ "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  echo "OpenSSL recipe requires macOS arm64" >&2
  exit 1
fi

openssl_version=3.5.9
openssl_sha256=603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a
workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
openssl_build="$workspace_root/native/build/openssl-macos14"
openssl_archive="$workspace_root/native/build/openssl-$openssl_version.tar.gz"
openssl_prefix="$openssl_build/install"
export MACOSX_DEPLOYMENT_TARGET=14.0

mkdir -p "$openssl_build"
curl --fail --location --retry 2 --output "$openssl_archive" \
  "https://github.com/openssl/openssl/releases/download/openssl-$openssl_version/openssl-$openssl_version.tar.gz"
printf '%s  %s\n' "$openssl_sha256" "$openssl_archive" | shasum -a 256 -c -
tar -xzf "$openssl_archive" -C "$openssl_build"
cd "$openssl_build/openssl-$openssl_version"
perl ./Configure darwin64-arm64-cc no-shared no-zlib \
  --prefix="$openssl_prefix" --openssldir="$openssl_prefix/ssl" \
  --libdir=lib -mmacosx-version-min=14.0
make -j4
make test HARNESS_JOBS=4
make install_sw
cp LICENSE.txt "$openssl_prefix/LICENSE.txt"
