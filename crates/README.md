# Rust workspace

This directory will contain the Rust workspace. The workspace is intentionally not initialized in this documentation-only scaffold; Phase 0 will add manifests and compileable crate boundaries together.

Dependency direction: `domain` is dependency-light; `connection-manager` depends on domain and engine API; adapters implement engine API; importers produce domain types; platform/networking are behind explicit interfaces. Storage and secrets are separate so secret material cannot leak into profile persistence.
