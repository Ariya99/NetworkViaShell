# ADR 0002: Modular monolith

- **Status:** Accepted
- **Decision:** Keep the initial product in one desktop application with separated Rust crates/modules.
- **Context:** This is a local desktop client; distributed services add operational complexity without a demonstrated need.
- **Consequences:** Boundaries are enforced through APIs and dependency direction, not network services. Engines remain replaceable adapters.
