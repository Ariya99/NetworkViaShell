# Contributing

- Branch from `main` with focused names such as `feat/ssh-local-forward`.
- Keep pull requests small and scoped to one milestone.
- Do not push directly to `main`.
- Include tests for parsing, lifecycle transitions, platform changes and failure cleanup.
- Never commit credentials, private keys, subscription URLs, user profiles or generated binaries.
- Validate external input, avoid shell interpolation, and redact secrets from logs.
- Document platform-specific behavior and rollback paths.

## Pull request checklist
- [ ] Scope and behavior are described.
- [ ] Tests cover success and failure paths.
- [ ] No secrets or personal connection data are included.
- [ ] Windows and Linux implications are considered.
- [ ] Cleanup/rollback is verified where networking state changes.
- [ ] Documentation is updated.
