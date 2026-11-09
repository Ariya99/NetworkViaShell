# ADR 0002: Modular monolith

- **Status:** Accepted
- **Decision:** One desktop application with separated Rust crates/modules.
- **Context:** This is a local desktop client; distributed services add complexity without a demonstrated need.
- **Consequences:** Boundaries are enforced through APIs and dependency direction. Engines remain replaceable adapters.
