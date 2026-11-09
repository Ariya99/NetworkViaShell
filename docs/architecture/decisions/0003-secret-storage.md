# ADR 0003: OS-backed secret storage

- **Status:** Accepted
- **Decision:** Persist secrets in OS credential stores; persist only opaque references in app data.
- **Context:** Profiles may contain SSH passwords, private-key passphrases, tokens and subscription credentials.
- **Consequences:** Platform adapters must handle unavailable/locked stores explicitly. Plaintext fallback is not allowed by default.
