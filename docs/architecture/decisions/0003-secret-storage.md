# ADR 0003: OS-backed secret storage

- **Status:** Accepted
- **Decision:** Persist secret values in the operating system credential store; persist only opaque references in application data.
- **Context:** Profiles may contain SSH passwords, private-key passphrases, tokens, and subscription credentials.
- **Consequences:** Platform adapters must handle unavailable/locked credential services explicitly. Plaintext fallback is not allowed by default.
