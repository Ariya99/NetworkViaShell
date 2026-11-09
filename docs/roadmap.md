# Delivery roadmap

## Phase 0 — Repository and build foundation
- [x] Architecture and module boundaries.
- [x] Tauri 2 + React + TypeScript desktop shell.
- [x] Rust workspace and lint configuration.
- [x] Windows/Linux CI checks defined.
- [x] Verify CI runs successfully on GitHub Actions.
- [ ] Add dependency/license inventory and release policy.

## Phase 1 — SSH MVP
- [x] SSH profile model: host, port, username, identity-file path, local SOCKS port.
- [x] Key/agent authentication via system OpenSSH (non-interactive; no password handling).
- [x] Safe process launch, startup readiness detection, and idempotent shutdown.
- [x] Local SOCKS5 endpoint and basic connection status UI.
- [ ] Integration tests with disposable SSH test server.
- [ ] Profile CRUD and persistence.
- [ ] Secure key-file picker and richer diagnostics.

## Phase 2 — Configuration manager
- [ ] Profile CRUD, import/export, strict validation, schema versioning.
- [ ] OS credential-store references and sanitized diagnostics.

## Phase 3 — Xray engine
- [ ] Version-pinned engine distribution and signature/hash verification.
- [ ] VLESS, VMess, Trojan, Shadowsocks mapping.
- [ ] Share-link and JSON parsing with fixtures and negative tests.

## Phase 4 — OS integration
- [ ] System proxy with prior-state capture and rollback.
- [ ] DNS/routing abstractions and privilege boundary.
- [ ] TUN mode after threat model and cleanup tests.

## Phase 5 — NPV compatibility research
- [ ] Obtain legitimate sample files and document provenance.
- [ ] Identify encoding/container/version without assumptions based on extension.
- [ ] Implement read-only parser only after format is understood.
- [ ] Add fixtures, malformed-input tests and unsupported-version behavior.

## Phase 6 — Additional engines
- [ ] WireGuard integration and platform capability checks.
- [ ] Subscription refresh, QR scanning and advanced diagnostics.

## Phase 7 — Production release
- [ ] Code signing strategy, reproducible builds, SBOM.
- [ ] Upgrade, rollback, crash recovery and uninstall cleanup tests.
- [ ] Security review and release documentation.

A phase is complete only when behavior is implemented, tested on supported platforms, documented, and observable in CI. A folder or interface stub is not evidence of a delivered feature.
