use std::path::PathBuf;
use std::process::Command;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// Checks if an Antigravity instance or language_server is already running.
pub fn is_antigravity_running() -> bool {
    let s = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );

    for (_pid, process) in s.processes() {
        let name = process.name().to_string_lossy().to_lowercase();
        if name.contains("antigravity") || name.contains("language_server") {
            return true;
        }
    }
    false
}

/// Locates the Antigravity binary on the system or uses the provided custom path.
pub fn resolve_antigravity_binary(custom_path: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = custom_path {
        if path.is_file() {
            return Ok(path);
        }
        return Err("The specified Antigravity executable path does not exist.".into());
    }

    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let candidate = PathBuf::from(local_app_data).join("Programs").join("Antigravity").join("Antigravity.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            let candidate = PathBuf::from(prog_files).join("Antigravity").join("Antigravity.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    #[cfg(unix)]
    {
        for candidate in ["/usr/bin/antigravity", "/usr/local/bin/antigravity", "/opt/antigravity/antigravity"] {
            let p = PathBuf::from(candidate);
            if p.is_file() {
                return Ok(p);
            }
        }
    }

    Err("Could not automatically locate Antigravity. Please specify the executable path.".into())
}

/// Launches Antigravity with proxy environment variables and loopback bypass flags.
pub fn launch_antigravity_preset(
    socks_port: u16,
    custom_path: Option<PathBuf>,
) -> Result<(), String> {
    if is_antigravity_running() {
        return Err(
            "Antigravity is already running. Please close all running Antigravity windows first so it can launch through the tunnel."
                .into(),
        );
    }

    let binary = resolve_antigravity_binary(custom_path)?;
    let socks_url = format!("socks5h://127.0.0.1:{}", socks_port);
    let bypass_env = "localhost,127.0.0.1,::1,*.local";

    let mut cmd = Command::new(&binary);

    // Loopback bypass flags for Electron/Chromium
    cmd.arg(format!("--proxy-server=socks5://127.0.0.1:{}", socks_port));
    cmd.arg("--proxy-bypass-list=<local>;localhost;127.0.0.1;[::1]");

    // Proxy environment variables for language server & child processes
    cmd.env("ALL_PROXY", &socks_url);
    cmd.env("all_proxy", &socks_url);
    cmd.env("HTTP_PROXY", &socks_url);
    cmd.env("http_proxy", &socks_url);
    cmd.env("HTTPS_PROXY", &socks_url);
    cmd.env("https_proxy", &socks_url);
    cmd.env("NO_PROXY", bypass_env);
    cmd.env("no_proxy", bypass_env);
    cmd.env("grpc_proxy", &socks_url);

    cmd.spawn().map_err(|e| format!("Failed to spawn Antigravity: {e}"))?;
    Ok(())
}
