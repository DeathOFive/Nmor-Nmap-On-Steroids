#!/usr/bin/env bash
set -euo pipefail
# Default no-sudo installation into the user's ~/.local/bin.
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
prefix="${NMOR_INSTALL_PREFIX:-$HOME/.local}"
install -Dm755 "$script_dir/nmor" "$prefix/bin/nmor"
echo "Installed: $prefix/bin/nmor"
echo 'Make sure your PATH contains ~/.local/bin, then run: nmor --help'
