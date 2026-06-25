# Nmor — Nmap on Roids

A raw-socket port scanner written in Rust. Same command-line muscle memory
as Nmap, faster underlying engine, a couple of UX upgrades Nmap doesn't
give you out of the box (no-sudo-after-install, adaptive rate control,
JSON output).

```
$ nmor -sS scanme.nmap.org -p 22,80,443

Nmor scan report for 45.33.32.156
PORT       STATE
22/tcp     open
80/tcp     open
443/tcp    closed

Nmor done: 1 host(s) scanned in 1.8s — 3 probes sent, 3 replies received
```

---

## Download

### Option A — Prebuilt binary (Linux x86_64, no compiling)

This is the fast path — no Rust, no build tools, nothing to install first.

1. Download the latest release: **[nmor-linux-x86_64.zip](../../releases/latest)**
2. Extract and install:

   ```bash
   unzip nmor-linux-x86_64.zip
   cd nmor_release
   ./install.sh
   ```

   The installer copies `nmor` to `/usr/local/bin` and grants it
   `CAP_NET_RAW` via `setcap`, so you can run SYN/ACK scans **without
   typing `sudo` every time** — already simpler than a stock Nmap install.

3. Confirm it works:

   ```bash
   nmor -sT scanme.nmap.org -p 22,80,443
   ```

Works on native Linux and on **WSL2** (Windows Subsystem for Linux). It
will *not* work on WSL1 — raw sockets need a real Linux kernel. Check with
`wsl -l -v`; if your distro says `VERSION 1`, convert it first:
`wsl --set-version <distro> 2`.

To remove it later: `./uninstall.sh` (from the same extracted folder).

### Option B — Build from source

For non-x86_64 systems, or if you just want to build it yourself:

```bash
git clone <this-repo-url>
cd nmor
cargo build --release
sudo ./target/release/nmor -sS 127.0.0.1
```

Requires a Rust toolchain — install one from [rustup.rs](https://rustup.rs)
if you don't have one. Any reasonably recent stable version works.

### Platform support

| Platform | Status |
|---|---|
| Linux (native), x86_64 |  fully supported, prebuilt binary available |
| WSL2 |  fully supported (same Linux binary) |
| Linux, other architectures |  build from source |
| WSL1 |  no raw sockets — not supported |
| Windows (native) |  not yet — needs a Npcap/WinPcap-based rewrite of the packet layer |
| macOS |  untested, likely needs minor portability fixes |

---

## Usage

If you've used Nmap, this should feel immediately familiar.

```bash
# SYN scan (the default technique, like Nmap's -sS) — needs root or setcap
nmor -sS scanme.nmap.org -p 22,80,443

# TCP connect scan — never needs root, good fallback on locked-down systems
nmor -sT scanme.nmap.org -p 1-1000

# ACK scan — maps which ports a firewall is actually blocking
nmor -sA 192.168.1.1 -p 1-1000

# SYN scan + lightweight banner/service grab on open ports
nmor -sSV 192.168.1.0/24 -p 1-1000

# Show *why* each port got its state (syn-ack / reset / no-response)
nmor -sS 10.0.0.1 -p 1-1000 --reason

# Aggressive timing, every port, write all output formats
nmor -sS 10.0.0.1 -T4 -p- -oA myscan
```

### Targets

Accepts anything Nmap does for simple targets: a single IP, a hostname
(resolved via DNS), a CIDR block, an IP range, or a comma-separated mix.

```bash
nmor -sS 10.0.0.1,10.0.0.0/24,192.168.1.1-50,scanme.nmap.org -p 80
```

### Ports (`-p`)

```bash
-p 80              # single port
-p 22,80,443       # list
-p 1-1024          # range
-p-                # all 65535 ports
-p-1024            # 1 through 1024
-p1024-            # 1024 through 65535
```

If `-p` isn't given, nmor scans its built-in top-100 most common ports.

### Scan techniques (`-s`)

| Flag | Technique | Needs root/setcap? |
|---|---|---|
| `-sS` | SYN scan (default) | yes |
| `-sT` | TCP connect scan | no |
| `-sA` | ACK scan (firewall mapping) | yes |
| `-sV` | banner/version grab (combine: `-sSV`) | no (runs after the main scan) |
| `-sU`, `-sN`, `-sF`, `-sX`, `-sn`, `-O` | — | not implemented yet; nmor tells you clearly instead of guessing |

You can't combine two *techniques* (`-sST` is rejected), but you can
combine a technique with `-sV` (`-sSV`, `-sTV`).

### Timing (`-T`) and rate control

```bash
-T0   # paranoid (very slow)
-T3   # normal (default)
-T5   # insane (very fast)

--min-rate 100      # floor for the adaptive controller
--max-rate 10000    # ceiling for the adaptive controller
```

Unlike Nmap's fixed timing model, nmor's sender uses an AIMD adaptive
controller that backs off automatically if reply rates drop — similar in
spirit to TCP congestion control. `--min-rate`/`--max-rate` override
whatever `-T` template you picked.

### Output formats

```bash
-oN file.txt     # human-readable, Nmap-style
-oG file.gnmap   # greppable
-oX file.xml     # structured XML
-oJ file.json    # JSON (nmor extension — real Nmap has no native JSON output)
-oA basename     # all four at once: basename.txt/.gnmap/.xml/.json
```

### Other compatibility flags

`-Pn`, `-n`, `-e/--interface` are accepted (so your old Nmap command lines
don't break) but currently no-ops — nmor never pings before scanning and
doesn't do reverse-DNS lookups anyway.

---

## Why nmor instead of Nmap?

- **Simpler privilege story**: one `setcap` call at install time, no
  `sudo` needed for every scan afterward.
- **Adaptive rate control** baked in, not a manual tuning exercise.
- **JSON output** natively, no XML-to-JSON conversion step.
- **Modern Rust codebase**: memory-safe packet parsing, no 20+ years of
  C/C++ legacy baggage.

## What it doesn't do (yet)

Being honest about the gap:

- No OS fingerprinting (`-O` is a no-op for now; a minimal TTL-based guess
  is planned, not full signature matching)
- No NSE-style scripting engine
- No UDP scanning
- IPv4 only — no IPv6 support yet
- `-sV` is a lightweight banner grab, not Nmap's full probe-database
  version detection

This a beta version, be free to test it out! :)
