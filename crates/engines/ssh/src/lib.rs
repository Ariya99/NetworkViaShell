pub mod host_key;
pub mod ipc;

use host_key::is_host_known;
use ipc::IpcSession;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    process::{Child, Command},
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time::{sleep, Instant},
};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TunnelHealth {
    Idle,
    Connecting,
    Connected,
    ConnectedInternetFailed,
    Reconnecting,
    Stopped,
    AuthFailed,
    HostKeyMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshStartError {
    HostUnverified { host: String, port: u16 },
    AuthenticationRejected(String),
    HostKeyMismatch(String),
    PortInUse(String),
    NetworkFailure(String),
    Timeout,
    Cancelled,
}

impl std::fmt::Display for SshStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HostUnverified { host, port } => write!(f, "HOST_UNVERIFIED:{}:{}", host, port),
            Self::AuthenticationRejected(msg) => write!(f, "AUTHENTICATION_FAILED: {}", msg),
            Self::HostKeyMismatch(msg) => write!(f, "HOST_KEY_MISMATCH: {}", msg),
            Self::PortInUse(msg) => write!(f, "PORT_IN_USE: {}", msg),
            Self::NetworkFailure(msg) => write!(f, "NETWORK_FAILURE: {}", msg),
            Self::Timeout => write!(f, "TIMEOUT: SOCKS5 listener failed to open within 15 seconds."),
            Self::Cancelled => write!(f, "CANCELLED: Connection cancelled by user."),
        }
    }
}

impl std::error::Error for SshStartError {}

/// Connection topology profile. Passwords and keys are never stored here.
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
                return Err("The selected SSH identity file does not exist or is not a file.".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ActiveContext {
    profile: SshProfile,
    known_hosts_path: PathBuf,
    askpass_executable: PathBuf,
    password: Option<Zeroizing<String>>,
}

struct TunnelState {
    child: Option<Child>,
    ipc_session: Option<IpcSession>,
    cancel_token: CancellationToken,
    context: Option<ActiveContext>,
}

#[derive(Clone)]
pub struct SshTunnelManager {
    inner: Arc<Mutex<TunnelState>>,
    health: Arc<RwLock<TunnelHealth>>,
    supervisor_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for SshTunnelManager {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TunnelState {
                child: None,
                ipc_session: None,
                cancel_token: CancellationToken::new(),
                context: None,
            })),
            health: Arc::new(RwLock::new(TunnelHealth::Idle)),
            supervisor_handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl SshTunnelManager {
    pub async fn get_health(&self) -> TunnelHealth {
        *self.health.read().await
    }

    /// Starts OpenSSH with application-owned known_hosts and secure askpass IPC.
    pub async fn start(
        &self,
        profile: SshProfile,
        known_hosts_path: PathBuf,
        askpass_executable: PathBuf,
        password: Option<String>,
    ) -> Result<String, SshStartError> {
        profile.validate().map_err(SshStartError::NetworkFailure)?;

        // Enforce application-owned known_hosts verification (TOFU)
        if !is_host_known(&known_hosts_path, &profile.host, profile.port) {
            return Err(SshStartError::HostUnverified {
                host: profile.host.clone(),
                port: profile.port,
            });
        }

        // Abort and join any existing supervisor task before starting anew
        self.abort_existing_supervisor().await;

        let cancel_token = CancellationToken::new();
        let password_zeroized = password.map(Zeroizing::new);
        let ctx = ActiveContext {
            profile: profile.clone(),
            known_hosts_path: known_hosts_path.clone(),
            askpass_executable: askpass_executable.clone(),
            password: password_zeroized.clone(),
        };

        // Prepare initial state without holding the lock across network startup
        {
            let mut state = self.inner.lock().await;
            if state.child.is_some() {
                return Err(SshStartError::NetworkFailure(
                    "An SSH tunnel is already running. Stop it before starting another.".into(),
                ));
            }
            state.cancel_token = cancel_token.clone();
            state.context = Some(ctx.clone());
        }

        *self.health.write().await = TunnelHealth::Connecting;

        // Perform connection startup without holding inner state lock
        let spawn_res = spawn_and_verify(
            &ctx.profile,
            &ctx.known_hosts_path,
            &ctx.askpass_executable,
            ctx.password.as_ref(),
            &cancel_token,
        )
        .await;

        match spawn_res {
            Ok((child, session, address)) => {
                {
                    let mut state = self.inner.lock().await;
                    if state.cancel_token.is_cancelled() {
                        let mut dying_child = child;
                        let _ = dying_child.kill().await;
                        let _ = dying_child.wait().await;
                        state.context = None;
                        *self.health.write().await = TunnelHealth::Stopped;
                        return Err(SshStartError::Cancelled);
                    }
                    state.child = Some(child);
                    state.ipc_session = session;
                }

                *self.health.write().await = TunnelHealth::Connected;

                // Spawn new supervised monitoring & auto-reconnect loop
                let manager_clone = self.clone();
                let supervisor = tokio::spawn(async move {
                    manager_clone.supervision_and_reconnect_loop().await;
                });
                *self.supervisor_handle.lock().await = Some(supervisor);

                Ok(address)
            }
            Err(err) => {
                let mut state = self.inner.lock().await;
                state.context = None;
                *self.health.write().await = match err {
                    SshStartError::AuthenticationRejected(_) => TunnelHealth::AuthFailed,
                    SshStartError::HostKeyMismatch(_) => TunnelHealth::HostKeyMismatch,
                    _ => TunnelHealth::Idle,
                };
                Err(err)
            }
        }
    }

    async fn abort_existing_supervisor(&self) {
        let mut handle_lock = self.supervisor_handle.lock().await;
        if let Some(handle) = handle_lock.take() {
            handle.abort();
            let _ = handle.await;
        }
    }

    /// Background supervision loop with structured error classification and exponential backoff.
    async fn supervision_and_reconnect_loop(&self) {
        let mut backoff = Duration::from_secs(1);
        let mut first_probe_failure: Option<Instant> = None;

        loop {
            sleep(Duration::from_secs(5)).await;

            let (cancel_token, ctx_opt) = {
                let state = self.inner.lock().await;
                (state.cancel_token.clone(), state.context.clone())
            };

            if cancel_token.is_cancelled() || ctx_opt.is_none() {
                break;
            }
            let ctx = ctx_opt.unwrap();

            // 1. Process Health & Exit Diagnostic Inspection
            let mut process_died = false;
            let mut fatal_error: Option<SshStartError> = None;

            {
                let mut state = self.inner.lock().await;
                if let Some(ref mut child) = state.child {
                    match child.try_wait() {
                        Ok(Some(_status)) => {
                            process_died = true;
                            // Capture stderr diagnostics to classify failure
                            let mut stderr_content = String::new();
                            if let Some(mut stderr) = child.stderr.take() {
                                let _ = stderr.read_to_string(&mut stderr_content).await;
                            }
                            let classified = classify_stderr(&stderr_content);
                            match classified {
                                SshStartError::AuthenticationRejected(_) => {
                                    fatal_error = Some(classified);
                                }
                                SshStartError::HostKeyMismatch(_) => {
                                    fatal_error = Some(classified);
                                }
                                _ => {
                                    // Normal connection drop / network failure: allow reconnect!
                                }
                            }
                        }
                        Ok(None) => {} // Still running
                        Err(_) => {
                            process_died = true;
                        }
                    }
                } else {
                    process_died = true;
                }

                if process_died {
                    state.child = None;
                    state.ipc_session = None;
                }
            }

            if process_died {
                if let Some(fatal) = fatal_error {
                    match fatal {
                        SshStartError::AuthenticationRejected(_) => {
                            *self.health.write().await = TunnelHealth::AuthFailed;
                        }
                        SshStartError::HostKeyMismatch(_) => {
                            *self.health.write().await = TunnelHealth::HostKeyMismatch;
                        }
                        _ => {}
                    }
                    let mut state = self.inner.lock().await;
                    state.context = None;
                    break;
                }

                // Supervised Reconnect with Exponential Backoff
                *self.health.write().await = TunnelHealth::Reconnecting;

                // Wait backoff duration while respecting cancellation
                let backoff_deadline = Instant::now() + backoff;
                let mut cancelled = false;
                while Instant::now() < backoff_deadline {
                    sleep(Duration::from_millis(200)).await;
                    if cancel_token.is_cancelled() {
                        cancelled = true;
                        break;
                    }
                }
                if cancelled {
                    break;
                }

                // Attempt reconnect without holding state lock across network startup
                let reconnect_res = spawn_and_verify(
                    &ctx.profile,
                    &ctx.known_hosts_path,
                    &ctx.askpass_executable,
                    ctx.password.as_ref(),
                    &cancel_token,
                )
                .await;

                match reconnect_res {
                    Ok((child, session, _)) => {
                        let mut state = self.inner.lock().await;
                        if state.cancel_token.is_cancelled() {
                            let mut dying = child;
                            let _ = dying.kill().await;
                            let _ = dying.wait().await;
                            break;
                        }
                        state.child = Some(child);
                        state.ipc_session = session;
                        *self.health.write().await = TunnelHealth::Connected;
                        backoff = Duration::from_secs(1);
                        first_probe_failure = None;
                    }
                    Err(SshStartError::AuthenticationRejected(_)) => {
                        *self.health.write().await = TunnelHealth::AuthFailed;
                        let mut state = self.inner.lock().await;
                        state.context = None;
                        break;
                    }
                    Err(SshStartError::HostKeyMismatch(_)) => {
                        *self.health.write().await = TunnelHealth::HostKeyMismatch;
                        let mut state = self.inner.lock().await;
                        state.context = None;
                        break;
                    }
                    Err(SshStartError::Cancelled) => {
                        break;
                    }
                    Err(_other_network_err) => {
                        // Increase backoff up to cap
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                }
                continue;
            }

            // 2. Active SOCKS5 + HTTP Egress Probe (every 10s)
            let socks_port = ctx.profile.local_socks_port;
            let probe_ok = verify_socks5_egress_probe(socks_port).await;

            if probe_ok {
                first_probe_failure = None;
                let mut h = self.health.write().await;
                if *h == TunnelHealth::ConnectedInternetFailed {
                    *h = TunnelHealth::Connected;
                }
            } else {
                if first_probe_failure.is_none() {
                    first_probe_failure = Some(Instant::now());
                }
                if let Some(start) = first_probe_failure {
                    // Only transition after 30 continuous seconds of probe failure
                    if start.elapsed() >= Duration::from_secs(30) {
                        *self.health.write().await = TunnelHealth::ConnectedInternetFailed;
                    }
                }
            }
        }
    }

    /// Stops the owned OpenSSH child process and aborts supervisor immediately.
    pub async fn stop(&self) -> Result<(), String> {
        let (mut child_opt, mut session_opt) = {
            let mut state = self.inner.lock().await;
            state.cancel_token.cancel();
            state.context = None;
            (state.child.take(), state.ipc_session.take())
        };

        drop(session_opt.take());

        if let Some(mut child) = child_opt.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        self.abort_existing_supervisor().await;
        *self.health.write().await = TunnelHealth::Stopped;
        Ok(())
    }
}

/// Spawns OpenSSH process, consumes startup stderr, and validates initial listener binding.
/// Does NOT hold the state lock during startup, enabling instant cancellation.
async fn spawn_and_verify(
    profile: &SshProfile,
    known_hosts_path: &PathBuf,
    askpass_executable: &PathBuf,
    password: Option<&Zeroizing<String>>,
    cancel_token: &CancellationToken,
) -> Result<(Child, Option<IpcSession>, String), SshStartError> {
    if cancel_token.is_cancelled() {
        return Err(SshStartError::Cancelled);
    }

    let bind_address = format!("127.0.0.1:{}", profile.local_socks_port);
    let probe = TcpListener::bind(&bind_address)
        .await
        .map_err(|e| SshStartError::PortInUse(format!("{e}")))?;
    drop(probe);

    let mut command = Command::new("ssh");
    command.args(build_arguments(profile, known_hosts_path, password.is_some()));
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());
    command.kill_on_drop(true);

    let mut session_opt = None;
    if let Some(pwd) = password {
        let session = IpcSession::start(pwd.to_string())
            .map_err(SshStartError::NetworkFailure)?;
        command.env("SSH_ASKPASS", askpass_executable);
        command.env("SSH_ASKPASS_REQUIRE", "force");
        command.env("NV_ASKPASS_PIPE", &session.pipe_name);
        command.env("NV_ASKPASS_TOKEN", &session.token);
        session_opt = Some(session);
    }

    let mut child = command.spawn().map_err(|e| {
        SshStartError::NetworkFailure(format!("Could not start OpenSSH client: {e}"))
    })?;

    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if cancel_token.is_cancelled() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(SshStartError::Cancelled);
        }

        if let Some(_exit_status) = child
            .try_wait()
            .map_err(|e| SshStartError::NetworkFailure(format!("{e}")))?
        {
            let mut stderr_content = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut stderr_content).await;
            }
            return Err(classify_stderr(&stderr_content));
        }

        if TcpStream::connect(&bind_address).await.is_ok() {
            return Ok((child, session_opt, bind_address));
        }

        if Instant::now() >= deadline {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(SshStartError::Timeout);
        }

        tokio::select! {
            _ = cancel_token.cancelled() => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(SshStartError::Cancelled);
            }
            _ = sleep(Duration::from_millis(100)) => {}
        }
    }
}

/// Classifies captured OpenSSH stderr into structured domain error variants.
fn classify_stderr(stderr: &str) -> SshStartError {
    let lower = stderr.to_lowercase();
    if lower.contains("permission denied") {
        SshStartError::AuthenticationRejected(stderr.trim().to_string())
    } else if lower.contains("host key verification failed")
        || lower.contains("remote host identification has changed")
    {
        SshStartError::HostKeyMismatch(stderr.trim().to_string())
    } else if lower.contains("address already in use") || lower.contains("cannot assign requested address") {
        SshStartError::PortInUse(stderr.trim().to_string())
    } else {
        SshStartError::NetworkFailure(if stderr.trim().is_empty() {
            "OpenSSH process terminated.".to_string()
        } else {
            stderr.trim().to_string()
        })
    }
}

/// Performs a genuine SOCKS5 handshake and HTTP probe through the local SOCKS proxy.
async fn verify_socks5_egress_probe(socks_port: u16) -> bool {
    let connect_fut = async {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", socks_port)).await.ok()?;

        // SOCKS5 Greeting: [Version 5, 1 Method, No Auth (0x00)]
        stream.write_all(&[0x05, 0x01, 0x00]).await.ok()?;
        let mut greeting_resp = [0u8; 2];
        stream.read_exact(&mut greeting_resp).await.ok()?;
        if greeting_resp != [0x05, 0x00] {
            return None;
        }

        // SOCKS5 Connect to 1.1.1.1:80 [0x05, 0x01 (connect), 0x00, 0x01 (IPv4), 1, 1, 1, 1, 0, 80]
        stream.write_all(&[0x05, 0x01, 0x00, 0x01, 1, 1, 1, 1, 0, 80]).await.ok()?;
        let mut connect_resp = [0u8; 10];
        stream.read_exact(&mut connect_resp).await.ok()?;
        if connect_resp[1] != 0x00 {
            return None;
        }

        // Send lightweight HTTP probe
        let req = b"GET /generate_204 HTTP/1.1\r\nHost: cp.cloudflare.com\r\nConnection: close\r\n\r\n";
        stream.write_all(req).await.ok()?;

        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.ok()?;
        if n > 0 && (buf.starts_with(b"HTTP/1.1 204") || buf.starts_with(b"HTTP/1.1 200") || buf.starts_with(b"HTTP/1.1 301")) {
            Some(())
        } else {
            None
        }
    };

    tokio::time::timeout(Duration::from_secs(4), connect_fut)
        .await
        .map(|opt| opt.is_some())
        .unwrap_or(false)
}

fn build_arguments(profile: &SshProfile, known_hosts_path: &PathBuf, has_password: bool) -> Vec<String> {
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
        "ExitOnForwardFailure=yes".into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        format!("UserKnownHostsFile={}", known_hosts_path.to_string_lossy()),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
        "-o".into(),
        "ServerAliveCountMax=3".into(),
    ];

    if has_password {
        args.push("-o".into());
        args.push("NumberOfPasswordPrompts=1".into());
    } else {
        args.push("-o".into());
        args.push("BatchMode=yes".into());
    }

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

    fn sample_profile() -> SshProfile {
        SshProfile {
            host: "ssh.example.com".into(),
            port: 22,
            username: "tunnel_user".into(),
            local_socks_port: 1080,
            identity_file: None,
        }
    }

    #[test]
    fn validates_correctly() {
        assert!(sample_profile().validate().is_ok());
    }

    #[test]
    fn enforces_known_hosts_file_and_batch_mode_flag() {
        let hosts_file = PathBuf::from("/tmp/nv_known_hosts");
        let args_key = build_arguments(&sample_profile(), &hosts_file, false);
        assert!(args_key.iter().any(|a| a == "BatchMode=yes"));
        assert!(args_key.iter().any(|a| a.starts_with("UserKnownHostsFile=")));

        let args_pwd = build_arguments(&sample_profile(), &hosts_file, true);
        assert!(!args_pwd.iter().any(|a| a == "BatchMode=yes"));
        assert!(args_pwd.iter().any(|a| a == "NumberOfPasswordPrompts=1"));
    }

    #[test]
    fn classifies_permission_denied_as_authentication_rejected() {
        let stderr = "user@example.com: Permission denied (publickey,password).";
        let err = classify_stderr(stderr);
        assert!(matches!(err, SshStartError::AuthenticationRejected(_)));
    }

    #[test]
    fn classifies_network_drop_as_network_failure_not_auth_failure() {
        let stderr = "Connection to 13.140.95.162 closed by remote host.";
        let err = classify_stderr(stderr);
        assert!(matches!(err, SshStartError::NetworkFailure(_)));
    }
}
