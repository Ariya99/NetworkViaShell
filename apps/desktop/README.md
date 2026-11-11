# NetworkViaShell desktop

Tauri 2 + React + TypeScript + Vite desktop application.

## Requirements

### Linux
- Linux Mint/Ubuntu
- Node.js 22+, pnpm 9+, Rust stable
- Tauri Linux prerequisites: https://v2.tauri.app/start/prerequisites/
- OpenSSH client (`ssh`)

### Windows
- Windows 10/11 x64
- Node.js 22+, pnpm 9+, Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools (Desktop development with C++)
- Microsoft Edge WebView2 Runtime
- NSIS (for the `.exe` installer)
- OpenSSH Client optional Windows feature (`ssh.exe` available on PATH)

## Development

Run from this directory:

```sh
pnpm install
pnpm tauri dev
```

## Build packages

Linux:

```sh
pnpm tauri build --bundles deb,appimage
```

Windows x64 (PowerShell):

```powershell
pnpm tauri build --bundles nsis
```

The Windows installer is written to `src-tauri/target/release/bundle/nsis/`. GitHub Actions builds it on `windows-latest` and publishes it as a downloadable workflow artifact named `NetworkViaShell-windows-x64-nsis`.

## Saved profiles

Profiles are stored as JSON in Tauri's application config directory. Linux uses owner-only directory/file permissions and atomic file replacement. Only the profile name, host, SSH port, username, local SOCKS port, and optional identity-file path are stored. Private-key contents and passwords are not stored.

## Connect

1. Ensure the server host key is already trusted in the current user's OpenSSH `known_hosts` file (`%USERPROFILE%\\.ssh\\known_hosts` on Windows, `~/.ssh/known_hosts` on Linux).
2. Ensure the OpenSSH client is installed and available as `ssh` on PATH.
3. Select an existing profile or enter connection details. Use a Windows path such as `C:\\Users\\you\\.ssh\\id_ed25519` for an identity file on Windows.
4. Save the profile if you want to reuse it.
5. Connect and use the displayed `127.0.0.1:PORT` as a SOCKS5 proxy in your browser/application.
6. Disconnect when finished.

The application does not alter system proxy settings, DNS, or routes.
