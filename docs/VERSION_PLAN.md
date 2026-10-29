# Development and merge plan

## v0.2.0: Linux-first GPLv3 source

- Linux supported release target; current scanner implemented with Tokio TCP connect sockets.
- CLI, targets, ports, async TCP scanning and discovery are modules shared by future platforms.
- Release with GPL-3.0-only, matching the existing repository's GPLv3 LICENSE.
- Do not overwrite the older Linux prebuilt ZIP in main until its original implementation is audited. Its README advertises raw SYN, ACK, output formats and no-sudo behavior that are absent from this source edition.

## v0.3.0: Integrate Windows branch

- Keep your Windows v0.2 work separately committed on `windows-v0.2.0`.
- Extract `cli`, `ports`, `targets`, `scanner/tcp` and output types into platform-neutral library modules.
- Use Linux-specific packet features under `#[cfg(target_os = "linux")]` as implemented and tested; use Windows-specific mechanisms behind a platform backend where necessary.
- Unify the CLI and run the same cross-platform integration tests on both OSes in GitHub Actions.
- Add a shared `service` detection module and structured output once ready.
- Merge via PR after resolving licensing (GPLv3) and feature parity, not by replacing one branch's README and source wholesale.

## v0.4.0: Vulnerability enrichment

- Confirmed service version fingerprinting, CPE mapping, and opt-in CVE data enrichment.
- Keep reported vulnerabilities separate from confirmed exploitable findings.
