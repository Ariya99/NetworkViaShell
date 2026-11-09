# Security baseline

- Never store passwords, private keys, tokens or subscription credentials in plaintext SQLite fields.
- Use Windows Credential Manager/DPAPI-backed facilities and Linux Secret Service where available. No silent plaintext fallback.
- Store opaque secret references in profiles; do not export secret values by default.
- Redact credentials, auth headers, share links and sensitive host metadata from logs/diagnostics.
- Do not invoke a shell to construct command strings; use discrete process arguments.
- Pin and verify downloaded engine binaries; record provenance and version.
- Treat links, QR payloads, JSON, subscriptions and NPV files as untrusted input with size/resource limits.
- Never auto-connect solely because a profile was imported.
- Capture original proxy, DNS, route and firewall state before changing it; restore app-owned changes on failure and shutdown.
- Require explicit consent before system-wide routing or TUN changes.
- Report suspected vulnerabilities privately to the repository owner.
