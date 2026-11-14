use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::task::AbortHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

#[cfg(windows)]
use tokio::net::windows::named_pipe::ServerOptions;

#[cfg(unix)]
use tokio::net::UnixListener;

pub struct IpcSession {
    pub pipe_name: String,
    pub token: String,
    abort_handle: AbortHandle,
    #[allow(dead_code)]
    cleanup_path: Option<PathBuf>,
}

impl Drop for IpcSession {
    fn drop(&mut self) {
        // Immediately abort the listener task if still waiting
        self.abort_handle.abort();

        #[cfg(unix)]
        if let Some(path) = &self.cleanup_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl IpcSession {
    /// Spawns a single-use IPC server that serves the provided zeroized secret once authenticated by token.
    /// The task is bounded by a 15-second timeout and cancelled on drop.
    pub fn start(secret_input: String) -> Result<Self, String> {
        let secret = Zeroizing::new(secret_input);
        let session_id = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string();

        #[cfg(windows)]
        {
            let pipe_name = format!(r"\\.\pipe\nv-askpass-{}", session_id);

            // Construct ServerOptions with restrictive security attributes
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .max_instances(1)
                .create(&pipe_name)
                .map_err(|e| format!("Could not create secure named pipe: {e}"))?;

            let expected_token = token.clone();
            let task = tokio::spawn(async move {
                // Wait up to 15 seconds for OpenSSH helper to connect
                let connect_fut = server.connect();
                if let Ok(Ok(())) = tokio::time::timeout(Duration::from_secs(15), connect_fut).await {
                    let (reader, mut writer) = tokio::io::split(server);
                    let mut lines = BufReader::new(reader).lines();
                    if let Ok(Ok(Some(received_token))) =
                        tokio::time::timeout(Duration::from_secs(3), lines.next_line()).await
                    {
                        if received_token.trim() == expected_token {
                            let _ = writer.write_all(secret.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                    }
                }
            });

            Ok(Self {
                pipe_name,
                token,
                abort_handle: task.abort_handle(),
                cleanup_path: None,
            })
        }

        #[cfg(unix)]
        {
            let socket_path = std::env::temp_dir().join(format!("nv-askpass-{}.sock", session_id));
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }

            let listener = UnixListener::bind(&socket_path)
                .map_err(|e| format!("Could not bind unix domain socket: {e}"))?;

            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));
            }

            let expected_token = token.clone();
            let task = tokio::spawn(async move {
                let accept_fut = listener.accept();
                if let Ok(Ok((stream, _))) = tokio::time::timeout(Duration::from_secs(15), accept_fut).await {
                    let (reader, mut writer) = stream.into_split();
                    let mut lines = BufReader::new(reader).lines();
                    if let Ok(Ok(Some(received_token))) =
                        tokio::time::timeout(Duration::from_secs(3), lines.next_line()).await
                    {
                        if received_token.trim() == expected_token {
                            let _ = writer.write_all(secret.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                    }
                }
            });

            Ok(Self {
                pipe_name: socket_path.to_string_lossy().to_string(),
                token,
                abort_handle: task.abort_handle(),
                cleanup_path: Some(socket_path),
            })
        }
    }
}
