# ADR 0001: Desktop application stack

- **Status:** Accepted
- **Decision:** Tauri 2, React, TypeScript and Rust/Tokio.
- **Context:** Windows and Linux need native process, credential and networking integration without a large bundled runtime.
- **Consequences:** Rust owns security-sensitive orchestration and platform operations. Frontend remains presentation-focused. Native dependencies and packaging must be validated early in CI.
