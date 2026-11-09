# Rust workspace

Rust workspace boundary. The workspace is initialized at the repository root. Keep domain dependency-light; connection-manager depends on domain and engine API; adapters implement engine API; importers produce domain types; platform/networking are behind explicit interfaces. Storage and secrets remain separate.
