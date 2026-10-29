#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
if [[ "$(uname -s)" != Linux ]] || [[ "$(uname -m)" != x86_64 ]]; then
  echo 'This packaging script requires Linux x86_64 (native or WSL2).' >&2
  exit 1
fi
command -v cargo >/dev/null || { echo 'Rust Cargo not found. Install Rust from rustup.rs' >&2; exit 1; }
command -v sha256sum >/dev/null || { echo 'sha256sum command missing.' >&2; exit 1; }
echo '[1/4] Rust formatting checks'
cargo fmt --all -- --check
echo '[2/4] Clippy warnings'
cargo clippy --all-targets -- -D warnings
echo '[3/4] Automated tests'
cargo test --all-targets
echo '[4/4] Optimized release build'
cargo build --release --locked
mkdir -p dist
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/nmor_release"
cp target/release/nmor README.md LICENSE "$stage/nmor_release/"
cp scripts/install-linux.sh scripts/uninstall-linux.sh "$stage/nmor_release/"
chmod 755 "$stage/nmor_release/nmor" "$stage/nmor_release/install-linux.sh" "$stage/nmor_release/uninstall-linux.sh"
version="$(awk -F '"' '/^version[[:space:]]*=/ {print $2; exit}' Cargo.toml)"
if [[ -z "$version" ]]; then
  echo 'Could not read package version from Cargo.toml' >&2
  exit 1
fi
archive="nmor-v${version}-linux-x86_64.tar.gz"
tar -czf "dist/$archive" -C "$stage" nmor_release
(cd dist && sha256sum "$archive" > SHA256SUMS.txt)
echo "Packaged: $root/dist/$archive"
echo "Checksum: $root/dist/SHA256SUMS.txt"
echo 'Note: this glibc build is for compatible x86_64 Linux distributions; test on release targets.'
