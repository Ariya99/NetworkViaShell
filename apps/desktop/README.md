# NetworkViaShell desktop

Tauri 2 + React + TypeScript + Vite desktop application.

## Requirements
- Linux Mint/Ubuntu (first release target)
- Node.js 22+, pnpm 9+, Rust stable
- Tauri Linux prerequisites: https://v2.tauri.app/start/prerequisites/
- OpenSSH client (`ssh`)

## Development
```sh
cd apps/desktop
pnpm install
pnpm tauri dev
```

## Build packages
```sh
pnpm tauri build --bundles deb,appimage
```

Artifacts are written under `src-tauri/target/release/bundle/`. A pushed version tag (for example `v0.1.0`) also triggers the Linux release workflow, which creates a draft GitHub release with .deb and AppImage packages.

## Saved profiles
Profiles are stored as JSON in Tauri's application config directory. Linux uses owner-only directory/file permissions and atomic file replacement. Only the profile name, host, SSH port, username, local SOCKS port, and optional identity-file path are stored. Private-key contents and passwords are not stored.

## Connect
1. Ensure the server host key is already trusted in `~/.ssh/known_hosts`.
2. Select an existing profile or enter connection details.
3. Save the profile if you want to reuse it.
4. Connect and use the displayed `127.0.0.1:PORT` as a SOCKS5 proxy in your browser/application.
5. Disconnect when finished.

The application does not alter system proxy settings, DNS, or routes.
