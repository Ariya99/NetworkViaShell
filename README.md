# NetworkViaShell

A lightweight desktop client for SSH-based local SOCKS5 tunnelling. The first release focuses on a reliable, simple SSH workflow.

> **Current milestone:** SSH SOCKS5 MVP. Linux Mint/Ubuntu is the first supported release target; Windows x64 builds are now available as an NSIS `.exe` installer through GitHub Actions.

## Current capabilities
- Tauri 2 desktop app with React and TypeScript.
- SSH key/agent authentication through the system OpenSSH client; no password collection.
- Local SOCKS5 forwarding bound to 127.0.0.1.
- Strict host-key verification and non-interactive SSH invocation.
- Start/stop lifecycle with startup readiness checks.
- Named connection profiles stored in the application config directory. Private-key contents are never copied into the app.
- Linux release workflow for .deb and AppImage packages (tag-triggered draft release).
- Windows x64 NSIS installer build (`.exe`) through the Windows build workflow.

## Linux build
Install Tauri Linux prerequisites from https://v2.tauri.app/start/prerequisites/, then:

```sh
cd apps/desktop
pnpm install
pnpm tauri dev
```

Create Linux packages locally with `pnpm tauri build --bundles deb,appimage`. Push a version tag such as `v0.1.0` to trigger the Linux draft release.

## Windows build
Install Node.js 22+, pnpm 9+, Rust stable with the MSVC toolchain, Microsoft C++ Build Tools, WebView2 Runtime, and NSIS. Then run in PowerShell:

```powershell
cd apps/desktop
pnpm install
pnpm tauri dev
pnpm tauri build --bundles nsis
```

The installer is written under `apps/desktop/src-tauri/target/release/bundle/nsis/`. GitHub Actions can build the same x64 installer on `windows-latest`; run **Windows build** manually or push a `v*` tag, then download the `NetworkViaShell-windows-x64-nsis` workflow artifact. The workflow artifact is separate from the Linux draft release.

## First-run requirements
- Install/enable the OpenSSH Client and ensure `ssh` is available on PATH.
- Add the SSH server's host key to the current user's OpenSSH `known_hosts` before connecting. On Windows this is normally `%USERPROFILE%\\.ssh\\known_hosts`; on Linux it is `~/.ssh/known_hosts`.
- Use an SSH key available on disk or through ssh-agent.
- Configure an application or browser to use SOCKS5 at the displayed `127.0.0.1:PORT` endpoint. The app does not change system proxy settings.

## Security and scope
- Saved profiles contain connection metadata and an optional identity-file path only, not private-key material or passwords.
- On Linux, profile storage uses the application config directory with owner-only directory/file permissions and atomic replacement.
- SSH host-key verification is not bypassed.
- No system-wide proxy, DNS, routing, VPN/TUN, Xray, or WireGuard functionality is included in 0.1.

## Validation status
CI checks the frontend and Rust workspace on Ubuntu and Windows. Passing CI or producing an installer does not replace a real-server acceptance test. Before calling 0.1 ready, verify a real SSH server, SOCKS5 traffic through the remote host, reconnect/disconnect behavior, and install/launch the relevant packages on clean supported machines.

See [desktop instructions](apps/desktop/README.md) and [delivery roadmap](docs/roadmap.md).
