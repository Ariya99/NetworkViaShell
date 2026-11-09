# Architecture overview

## High-level shape
A Rust modular monolith behind a Tauri 2 desktop shell. React + TypeScript owns presentation; Rust owns connection state, configuration validation, process lifecycle, secret access, and privileged platform integration.

```text
React UI
  └── Tauri commands / events
       ├── Application domain
       ├── Connection manager → Engine API → OpenSSH / Xray / WireGuard adapters
       ├── Config import pipeline
       ├── Networking / platform adapters
       ├── Storage (non-secret data)
       ├── OS credential store
       └── Monitoring + diagnostics
```

## Dependency rules
- UI depends on typed Tauri commands/events, not engine internals.
- Engines implement a narrow common interface and do not own application state.
- Connection manager coordinates state transitions and cleanup.
- Importers parse untrusted input into validated domain models; they never start engines.
- Storage persists non-secret metadata. Secrets are referenced by opaque credential IDs.
- Platform modules encapsulate OS-specific proxy, route, DNS, firewall, and privilege operations.

## Connection lifecycle
Model connections explicitly: `Disconnected → Preparing → Connecting → Connected → Stopping → Disconnected`, with `Failed` retaining a sanitized error. Register compensating cleanup before each startup side effect. Stop must be idempotent, including after partial startup.

## First vertical slice
1. Create and validate an SSH profile.
2. Resolve credentials from the OS credential store or interactive prompt.
3. Start OpenSSH without shell interpolation and with controlled arguments.
4. Establish a local SOCKS5 listener using dynamic forwarding.
5. Verify readiness before reporting Connected.
6. On stop, terminate the process, close listeners, and restore app-owned state.

See the module READMEs and decision records in this directory.
