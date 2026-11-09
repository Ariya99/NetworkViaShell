# Architecture overview

A Rust modular monolith behind a Tauri 2 desktop shell. React + TypeScript owns presentation; Rust owns connection state, configuration validation, process lifecycle, secret access, and privileged platform integration.

```text
React UI → Tauri commands/events → domain + connection manager
                              → engine API → OpenSSH / Xray / WireGuard
                              → config import pipeline
                              → networking + platform adapters
                              → non-secret storage + OS credential store
                              → monitoring + diagnostics
```

## Dependency rules
- UI depends on typed Tauri commands/events, not engine internals.
- Engines implement a narrow common interface and do not own application state.
- Connection manager coordinates state transitions and cleanup.
- Importers parse untrusted input into validated domain models; they never start engines.
- Storage persists non-secret metadata. Secrets use opaque credential IDs.
- Platform modules encapsulate OS-specific proxy, route, DNS, firewall and privilege operations.

## Connection lifecycle
`Disconnected → Preparing → Connecting → Connected → Stopping → Disconnected`, with `Failed` retaining a sanitized error. Register compensating cleanup before each startup side effect. Stop must be idempotent, including after partial startup.

## First vertical slice
1. Validate an SSH profile.
2. Resolve credentials through OS credential storage or interactive prompt.
3. Start OpenSSH without shell interpolation.
4. Establish local SOCKS5 via dynamic forwarding.
5. Verify readiness before reporting Connected.
6. Stop process/listeners and restore app-owned state.
