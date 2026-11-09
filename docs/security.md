# Security baseline

## Sensitive data
- Never store passwords, private keys, tokens, or subscription credentials in plaintext SQLite fields.
- Use Windows Credential Manager / DPAPI-backed facilities and Linux Secret Service where available. Define explicit behavior when unavailable; no silent plaintext fallback.
- Store opaque secret references in profiles. Never serialize secret values into exports by default.
- Redact credentials, auth headers, share links, and sensitive host metadata from logs and diagnostics.

## Process execution
- Do not invoke a shell to construct command strings; use discrete process arguments.
- Avoid credentials in command-line arguments and environment variables where safer supported mechanisms exist.
- Pin and verify downloaded engine binaries before execution; record provenance and version.
- Restrict writable directories and validate paths against traversal and binary substitution.

## Imported configuration
- Treat share links, QR payloads, JSON, subscriptions, and NPV files as untrusted.
- Enforce size, nesting, timeout, and resource limits.
- Validate schemes, ports, addresses, and required fields before runtime configuration.
- Never auto-connect solely because a profile was imported.

## Network state
- Capture original proxy, DNS, route, and firewall state before changing it.
- Track only application-owned changes; restore them on normal stop, startup failure, crash recovery, and uninstall where feasible.
- Require explicit consent before system-wide routing or TUN changes.
- Surface actionable diagnostics if cleanup cannot be verified.

Suspected vulnerabilities should be reported privately to the repository owner rather than through public exploit details.
