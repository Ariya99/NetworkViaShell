# Contributing

## Workflow
- Branch from `main` using focused names such as `feat/ssh-local-forward`.
- Keep pull requests small, reviewable, and scoped to one milestone.
- Do not push directly to `main`.
- Include tests for parsing, lifecycle transitions, platform changes, and failure cleanup.
- Never commit credentials, private keys, subscription URLs, user profiles, or generated binaries.

## Engineering expectations
- Rust code must be formatted and pass Clippy.
- Validate external input at module boundaries.
- Avoid shell interpolation; pass subprocess arguments separately.
- Redact secrets from logs and diagnostic exports.
- Document platform-specific behavior and rollback paths.

## Pull request checklist
- [ ] Scope and behavior are described.
- [ ] Tests cover success and failure paths.
- [ ] No secrets or personal connection data are included.
- [ ] Windows and Linux implications are considered.
- [ ] Cleanup/rollback behavior is verified where networking state changes.
- [ ] Documentation is updated.
