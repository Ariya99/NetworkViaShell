# Real SSH acceptance test (Linux)

This is a manual release gate; CI cannot verify access to a user's private SSH server.

## Prerequisites
- A reachable SSH server and a user authorized for public-key authentication.
- The matching private key available locally or through ssh-agent.
- The server's verified host key in `~/.ssh/known_hosts`.
- OpenSSH client and curl installed.

Do not disable host-key checking to make a test pass.

## Test
1. Launch NetworkViaShell.
2. Enter the server host, SSH port, username, local SOCKS port (for example 1080), and optional private-key file path.
3. Save the profile, close the application, reopen it, and confirm the profile remains available with its values.
4. Connect. Confirm the UI reports Connected and shows `127.0.0.1:1080`.
5. In a terminal, run:

   ```sh
   curl --fail --show-error --socks5-hostname 127.0.0.1:1080 https://api.ipify.org
   ```

   The returned public IP should be the remote server's egress IP, not the local network's public IP.
6. Disconnect in the app. Confirm the local SOCKS port is no longer listening:

   ```sh
   ss -ltn | grep ':1080' || echo "SOCKS listener stopped"
   ```

7. Reconnect, then stop the SSH server/network connection and confirm the UI surfaces the failure. Restore connectivity and verify a clean reconnect.
8. Repeat using the .deb and AppImage installations on a clean supported Linux machine.

## Record results
Record Linux distribution/version, app version, SSH server OS/OpenSSH version, authentication method, SOCKS egress IP (do not publish it), and pass/fail for each step. Never attach private keys, passwords, or unredacted server addresses to public issues.
