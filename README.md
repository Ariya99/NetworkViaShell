# NetworkViaShell

Cross-platform desktop client for secure network tunnelling, starting with SSH and designed for additional protocol engines.

> **Status:** Tauri/Rust foundation in progress. No connection engine is implemented yet.

## Product direction
- Windows and Linux desktop support.
- First milestone: SSH local SOCKS5 forwarding, saved profiles, safe lifecycle, logs and diagnostics.
- Later: Xray protocols (VLESS, VMess, Trojan, Shadowsocks) and WireGuard.
- Imports: manual profiles, share links, QR, subscriptions, and eventually validated NPV formats.
- Stack: Tauri 2, React, TypeScript, Rust/Tokio modular monolith.

## Repository map
- `apps/desktop`: Tauri desktop application and React frontend.
- `crates/domain`: protocol-neutral models and invariants.
- `crates/connection-manager`: connection lifecycle and orchestration.
- `crates/engines`: SSH, Xray, WireGuard adapters and engine API.
- `crates/config-import`: links, QR, JSON, subscription and NPV parsers.
- `crates/networking`: local proxy, system proxy, TUN, DNS and routing.
- `crates/storage`: non-secret profile/settings persistence.
- `crates/secrets`: OS-backed credential storage.
- `crates/platform`: Windows/Linux integration.
- `crates/monitoring`, `diagnostics`: metrics, logs and support bundles.
- `docs`: architecture, roadmap, security and decisions.

## Engineering principles
1. Keep `main` releasable; use focused branches and pull requests.
2. Keep credentials out of SQLite, logs, command-line arguments, and Git.
3. Treat connection startup/shutdown as transactions with compensating cleanup.
4. Restore prior system proxy, routes and DNS state after app-owned changes.
5. Treat imported configuration as untrusted input.
6. Do not claim NPV compatibility until fixtures and tests prove it.
7. Prefer a modular monolith; avoid distributed services without demonstrated need.

## Development
See [desktop app instructions](apps/desktop/README.md), [architecture](docs/architecture/README.md), [roadmap](docs/roadmap.md), [security](docs/security.md), and [contributing](CONTRIBUTING.md).

This repository is under active development. The current desktop screen is a shell only; it does not create tunnels or store connection profiles.
