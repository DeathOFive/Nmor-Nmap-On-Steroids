#!/usr/bin/env bash
set -euo pipefail
prefix="${NMOR_INSTALL_PREFIX:-$HOME/.local}"
rm -f -- "$prefix/bin/nmor"
echo "Removed: $prefix/bin/nmor"
