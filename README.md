# Nmor — Nmap On Roids (v0.2.0, Linux source edition)

**Linux-first** Rust TCP port scanner. Current verified design scope is asynchronous TCP-connect scanning, TCP-based host discovery, IPv4/IPv6 CIDR target expansion, target-file input, and a global concurrency limit. Release candidate source, **not yet compiled in the packaging environment**: run the automated tests and GitHub Actions checks before distributing binaries.

**License: GNU GPL-3.0-only.** See [`LICENSE`](LICENSE). This matches the GPLv3 license in the existing [DeathOFive/Nmor-Nmap-On-Steroids](https://github.com/DeathOFive/Nmor-Nmap-On-Steroids) repository.

## Linux requirements

- Linux x86_64, including Debian/Ubuntu, Parrot, and **WSL2**. Other Linux CPU architectures can build from source but the provided packaging script targets x86_64.
- A current stable Rust toolchain (`rustup` and Cargo); GCC / build essentials on Linux.
- No root or packet-capture privileges for `-sT`: the scanner uses ordinary TCP sockets.
- Python 3 only for the optional local test listener.

### Setup (Linux terminal or VS Code attached to WSL)

```bash
# On Debian / Ubuntu / WSL2 Debian
sudo apt update && sudo apt install -y build-essential pkg-config python3 git curl
# Install Rust using the official rustup instructions at https://rustup.rs/
source "$HOME/.cargo/env"
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
./target/release/nmor --version
```

If rustfmt or Clippy is missing, install components using `rustup component add rustfmt clippy`.

### Test a known-open local TCP port

Terminal A:

```bash
bash scripts/test-local.sh
```

Terminal B:

```bash
cargo run -- -sT -Pn -p 8080 127.0.0.1
cargo run -- -sn -PS8080 127.0.0.1
cargo run -- -sT -Pn -p 8080 127.0.0.1 127.0.0.1/32
```

Look for `8080/tcp open`, a TCP discovery response, and `Targets: 1` for the duplicate target. Stop the listener in terminal A with Ctrl+C.

### Commands

```bash
nmor -sT -Pn -p 22,80,443 127.0.0.1
nmor -sT -Pn -p 1-1024 192.168.1.1
nmor -sn -PS22,80,443 192.168.1.0/28
nmor -iL examples/targets.txt -Pn -p 80,443
nmor -sT -Pn -p- --concurrency 512 127.0.0.1
```

| Option | Behavior |
|---|---|
| `-sT` | Asynchronous TCP connect scan (the only scan engine in this source) |
| `-Pn` | Skip discovery |
| `-sn` | Discovery-only using TCP probes |
| `-PS22,80` | TCP discovery probe ports |
| `-p 80,443` / `-p-` | Specify ports / all ports |
| `-iL` | Read targets from a file |
| `--concurrency 256` | Global maximum simultaneous TCP sockets |
| `--host-concurrency 16` | Maximum concurrent target tasks |
| `--timeout 1000` | TCP connect timeout in milliseconds |
| `--max-hosts 4096` | Maximum number of expanded IP addresses |

TCP refusal counts as a responsive host; TCP timeout is inconclusive. Closed and filtered TCP ports are summarized but only open ports are listed. Overlapping targets are deduplicated.

### Package a distributable Linux binary

```bash
bash scripts/package-linux.sh
```

After checks pass, expect:

```text
dist/nmor-v0.2.0-linux-x86_64.tar.gz
dist/SHA256SUMS.txt
```

Validate checksum and install to `~/.local/bin` without root:

```bash
cd dist
sha256sum -c SHA256SUMS.txt
tar -xzf nmor-v0.2.0-linux-x86_64.tar.gz
cd nmor_release
bash install-linux.sh
~/.local/bin/nmor --help
```

Linux binaries compiled against glibc aren't guaranteed to work on every distribution; test on the target systems. `install-linux.sh` does **not** call `setcap`: raw packet privileges are unnecessary for TCP-connect scanning.

### Current limitations / relationship to earlier repository

The existing repository README describes an earlier Linux *binary* featuring SYN/ACK techniques, adaptive rate control, and multiple output formats. The main branch currently does not publish that binary's original Rust source, so this source edition **does not claim those features**. Keep the earlier binary downloadable as a legacy build until its source and behavior can be reconciled with the GPL-licensed code in this branch.

This release has **no** SYN (`-sS`), ACK (`-sA`), UDP, ICMP, `-sV`, OS fingerprinting, native JSON, or CVE matching. It is not yet faster than Nmap based on published benchmarks. Unsupported flags should not be advertised as implemented.

See [`docs/VERSION_PLAN.md`](docs/VERSION_PLAN.md) for the eventual Linux/Windows merge in v0.3.0.
