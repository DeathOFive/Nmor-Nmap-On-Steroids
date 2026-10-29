#!/usr/bin/env bash
set -euo pipefail
# In another terminal: cargo run -- -sT -Pn -p 8080 127.0.0.1
exec python3 -m http.server 8080 --bind 127.0.0.1
