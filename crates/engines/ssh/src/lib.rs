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
            || !self.host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b':' | b'%' | b'_'))
        {
            return Err("Enter a valid SSH host name or IP address.".into());
        }
        if self.username.is_empty()
            || self.username.starts_with('-')
            || self.username.len() > 64
            || !self.username
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

pub struct SshTunnelManager {
    child: Mutex<Option<Child>>,
    ssh_program: PathBuf,
    startup_timeout: Duration,
}

impl Default for SshTunnelManager {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            ssh_program: PathBuf::from("ssh"),
            startup_timeout: STARTUP_TIMEOUT,
        }
    }
}

impl SshTunnelManager {
    /// Starts OpenSSH and waits until the local dynamic-forwarding listener is ready.
    pub async fn start(&self, profile: SshProfile) -> Result<String, String> {
        profile.validate()?;
        let mut running = self.child.lock().await;
        if running.is_some() {
            return Err("An SSH tunnel is already running. Stop it before starting another.".into());
        }

        let bind_address = format!("127.0.0.1:{}", profile.local_socks_port);
        let probe = TcpListener::bind(&bind_address)
            .await
            .map_err(|_| "The requested local SOCKS port is already in use.".to_string())?;
        drop(probe);

        let mut command = Command::new(&self.ssh_program);
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

        let deadline = Instant::now() + self.startup_timeout;
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

    #[cfg(all(test, unix))]
    fn with_ssh_program(ssh_program: PathBuf, startup_timeout: Duration) -> Self {
        Self {
            child: Mutex::new(None),
            ssh_program,
            startup_timeout,
        }
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

    #[cfg(unix)]
    mod lifecycle {
        use super::*;
        use std::{
            fs,
            net::TcpListener as StdTcpListener,
            os::unix::fs::PermissionsExt,
            sync::atomic::{AtomicU64, Ordering},
        };

        static NEXT_SCRIPT: AtomicU64 = AtomicU64::new(0);

        fn fake_ssh(body: &str) -> PathBuf {
            let id = NEXT_SCRIPT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "network-via-shell-fake-ssh-{}-{id}.sh",
                std::process::id()
            ));
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(&path, permissions).unwrap();
            path
        }

        fn free_port() -> u16 {
            StdTcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        }

        fn listener_script(seconds: &str) -> String {
            format!(
                r#"port=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "-D" ]; then port=$(printf '%s' "$argument" | cut -d: -f2); fi
  previous="$argument"
done
exec python3 -c 'import socket,sys,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); s.bind(("127.0.0.1",int(sys.argv[1]))); s.listen(8); time.sleep(float(sys.argv[2]))' "$port" "{seconds}""#
            )
        }

        #[tokio::test]
        async fn rejects_an_occupied_local_port_without_spawning_ssh() {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let script = fake_ssh("exit 0");
            let manager =
                SshTunnelManager::with_ssh_program(script.clone(), Duration::from_millis(100));
            let mut profile = valid_profile();
            profile.local_socks_port = port;
            let result = manager.start(profile).await;
            assert!(result.unwrap_err().contains("already in use"));
            drop(listener);
            fs::remove_file(script).unwrap();
        }

        #[tokio::test]
        async fn start_rejects_second_tunnel_and_stop_is_idempotent() {
            let port = free_port();
            let script = fake_ssh(&listener_script("30"));
            let manager =
                SshTunnelManager::with_ssh_program(script.clone(), Duration::from_secs(2));
            let mut profile = valid_profile();
            profile.local_socks_port = port;
            assert_eq!(
                manager.start(profile.clone()).await.unwrap(),
                format!("127.0.0.1:{port}")
            );
            assert!(manager.start(profile).await.unwrap_err().contains("already running"));
            manager.stop().await.unwrap();
            manager.stop().await.unwrap();
            assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
            fs::remove_file(script).unwrap();
        }

        #[tokio::test]
        async fn stop_handles_an_unexpectedly_exited_ssh_process() {
            let port = free_port();
            let script = fake_ssh(&listener_script("0.3"));
            let manager =
                SshTunnelManager::with_ssh_program(script.clone(), Duration::from_secs(2));
            let mut profile = valid_profile();
            profile.local_socks_port = port;
            manager.start(profile).await.unwrap();
            sleep(Duration::from_millis(450)).await;
            manager.stop().await.unwrap();
            assert!(TcpStream::connect(("127.0.0.1", port)).await.is_err());
            fs::remove_file(script).unwrap();
        }

        #[tokio::test]
        async fn reports_an_early_ssh_process_exit() {
            let script = fake_ssh("exit 17");
            let manager =
                SshTunnelManager::with_ssh_program(script.clone(), Duration::from_secs(1));
            let result = manager.start(valid_profile()).await;
            assert!(result.unwrap_err().contains("exited before"));
            fs::remove_file(script).unwrap();
        }

        #[tokio::test]
        async fn startup_timeout_kills_the_unresponsive_child() {
            let script = fake_ssh("exec python3 -c 'import time; time.sleep(30)'");
            let manager =
                SshTunnelManager::with_ssh_program(script.clone(), Duration::from_millis(150));
            let result = manager.start(valid_profile()).await;
            assert!(result.unwrap_err().contains("Timed out"));
            fs::remove_file(script).unwrap();
        }
    }
}
