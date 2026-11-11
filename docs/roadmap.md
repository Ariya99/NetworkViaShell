# Delivery roadmap

## 0.1 — SSH SOCKS5 desktop MVP
- [x] Tauri 2 + React + TypeScript desktop shell.
- [x] SSH key/agent authentication through system OpenSSH.
- [x] Loopback-only SOCKS5 forwarding, strict host-key checking, start/stop lifecycle.
- [x] Named profile save/load/update/delete with local persistence.
- [x] Linux .deb and AppImage draft-release workflow.
- [x] Windows x64 NSIS .exe installer workflow.
- [ ] Pass CI on the delivery branch.
- [ ] Verify a real remote SSH server and end-to-end SOCKS5 traffic on Linux and Windows.
- [ ] Install and smoke-test .deb and AppImage on clean Linux Mint/Ubuntu.
- [ ] Install and smoke-test the NSIS .exe on a clean Windows 10/11 x64 machine.
- [ ] Publish the reviewed 0.1.0 release.

## Explicitly out of scope for 0.1
- System proxy configuration, DNS/routing changes, TUN/VPN mode.
- Xray protocols, WireGuard, subscriptions, QR imports.
- Password storage, private-key import/storage, advanced diagnostics.

## Release acceptance

A release is ready only when profile persistence survives app restart, a real SSH server accepts key authentication, SOCKS5 traffic exits through that server, disconnect closes the local listener/process, and the relevant installers install and launch on clean supported machines.

Do not mark manual acceptance items complete based only on CI or a successful package build.
