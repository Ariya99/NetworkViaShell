use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    process::{Child, Command},
    sync::Mutex,
    time::{sleep, Instant},
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

/// A small SSH profile. Private key material stays in the SSH agent or on disk.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProfile {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub local_socks_port: u16,
    #[serde(default)]
    pub identity_file: Option<PathBuf>,
}

impl SshProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty()
            || self.host.starts_with('-')
            || !self
                .host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b':' | b'%' | b'_'))
        {
            return Err("Enter a valid SSH host name or IP address.".into());
        }
        if self.username.is_empty()
            || self.username.starts_with('-')
            || self.username.len() > 64
            || !self
                .username
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
        {
            return Err("Enter a valid SSH username.".into());
        }
        if self.port == 0 {
            return Err("SSH port must be between 1 and 65535.".into());
        }
        if self.local_socks_port == 0 {
            return Err("Local SOCKS port must be between 1 and 65535.".into());
        }
        if let Some(path) = &self.identity_file {
            if !path.is_file() {
                return Err(
                    "The selected SSH identity file does not exist or is not a file.".into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct SshTunnelManager {
    child: Mutex<Option<Child>>,
}

impl SshTunnelManager {
    /// Starts OpenSSH and waits until the local dynamic-forwarding listener is ready.
    pub async fn start(&self, profile: SshProfile) -> Result<String, String> {
        profile.validate()?;
        let mut running = self.child.lock().await;
        if running.is_some() {
            return Err(
                "An SSH tunnel is already running. Stop it before starting another.".into(),
            );
        }

        let bind_address = format!("127.0.0.1:{}", profile.local_socks_port);
        let probe = TcpListener::bind(&bind_address)
            .await
            .map_err(|_| "The requested local SOCKS port is already in use.".to_string())?;
        drop(probe);

        let mut command = Command::new("ssh");
        command
            .args(build_arguments(&profile))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|_| {
            "Could not start OpenSSH. Install the OpenSSH client and ensure ssh is on PATH."
                .to_string()
        })?;

        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if child
                .try_wait()
                .map_err(|_| "Could not inspect the OpenSSH process.".to_string())?
                .is_some()
            {
                return Err(
                    "OpenSSH exited before the SOCKS listener became ready. Check the host, key/agent, and known_hosts entry."
                        .into(),
                );
            }
            if TcpStream::connect(&bind_address).await.is_ok() {
                if child
                    .try_wait()
                    .map_err(|_| "Could not inspect the OpenSSH process.".to_string())?
                    .is_some()
                {
                    return Err("OpenSSH exited during tunnel startup.".into());
                }
                *running = Some(child);
                return Ok(bind_address);
            }
            if Instant::now() >= deadline {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err("Timed out waiting for the local SOCKS listener.".into());
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Stops the owned OpenSSH child. Repeated calls are harmless.
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.child.lock().await;
        if let Some(mut child) = running.take() {
            let already_exited = child
                .try_wait()
                .map_err(|_| "Could not inspect the OpenSSH process.".to_string())?
                .is_some();
            if !already_exited {
                child
                    .kill()
                    .await
                    .map_err(|_| "Could not stop the OpenSSH process.".to_string())?;
                let _ = child.wait().await;
            }
        }
        Ok(())
    }
}

fn build_arguments(profile: &SshProfile) -> Vec<String> {
    let mut args = vec![
        "-F".into(),
        if cfg!(windows) {
            "NUL".into()
        } else {
            "/dev/null".into()
        },
        "-N".into(),
        "-T".into(),
        "-D".into(),
        format!("127.0.0.1:{}", profile.local_socks_port),
        "-p".into(),
        profile.port.to_string(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
        "-o".into(),
        "ServerAliveCountMax=3".into(),
    ];
    if let Some(identity_file) = &profile.identity_file {
        args.push("-i".into());
        args.push(identity_file.to_string_lossy().into_owned());
        args.extend(["-o".into(), "IdentitiesOnly=yes".into()]);
    }
    args.push(format!("{}@{}", profile.username, profile.host));
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_profile() -> SshProfile {
        SshProfile {
            host: "example.com".into(),
            port: 22,
            username: "tunnel_user".into(),
            local_socks_port: 1080,
            identity_file: None,
        }
    }

    #[test]
    fn accepts_a_valid_profile() {
        assert!(valid_profile().validate().is_ok());
    }

    #[test]
    fn rejects_option_injection_in_host_and_username() {
        let mut profile = valid_profile();
        profile.host = "-oProxyCommand=evil".into();
        assert!(profile.validate().is_err());
        let mut profile = valid_profile();
        profile.username = "-oProxyCommand=evil".into();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_zero_ports() {
        let mut profile = valid_profile();
        profile.port = 0;
        assert!(profile.validate().is_err());
        let mut profile = valid_profile();
        profile.local_socks_port = 0;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn arguments_are_separate_and_enforce_host_key_checking() {
        let args = build_arguments(&valid_profile());
        assert!(args.iter().any(|arg| arg == "127.0.0.1:1080"));
        assert!(args.iter().any(|arg| arg == "tunnel_user@example.com"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "-o" && pair[1] == "StrictHostKeyChecking=yes"));
    }
}
