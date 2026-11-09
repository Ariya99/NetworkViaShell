# ADR 0001: Desktop application stack

- **Status:** Accepted for initial implementation
- **Decision:** Tauri 2, React, TypeScript, and Rust/Tokio.
- **Context:** The client targets Windows and Linux and needs native process, credential, and networking integration without a large bundled runtime.
- **Consequences:** Rust owns security-sensitive orchestration and platform operations. Frontend remains presentation-focused. Native dependencies and packaging must be validated early in CI.
