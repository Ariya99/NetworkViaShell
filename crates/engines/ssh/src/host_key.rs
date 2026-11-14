use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyInfo {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String,
    pub raw_entry: String,
}

/// Checks if the host:port combination is already recorded in the application's known_hosts file.
pub fn is_host_known(known_hosts_path: &Path, host: &str, port: u16) -> bool {
    if !known_hosts_path.exists() {
        return false;
    }
    let content = match fs::read_to_string(known_hosts_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let target_standard = format!("[{}]:{}", host, port);
    let target_default = if port == 22 { host } else { "" };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(host_part) = line.split_whitespace().next() {
            for entry in host_part.split(',') {
                if entry == target_standard || (!target_default.is_empty() && entry == target_default) {
                    return true;
                }
            }
        }
    }
    false
}

/// Probes the remote host key and computes its SHA256 fingerprint using ssh-keyscan and ssh-keygen.
/// This probe output is treated as strictly UNTRUSTED until approved by the user.
pub async fn probe_host_key(host: &str, port: u16) -> Result<HostKeyInfo, String> {
    let output = Command::new("ssh-keyscan")
        .args([
            "-p",
            &port.to_string(),
            "-T",
            "5",
            "-t",
            "ed25519,ecdsa,rsa",
            host,
        ])
        .output()
        .await
        .map_err(|e| format!("Could not execute ssh-keyscan: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .ok_or_else(|| "No host key received from the remote server.".to_string())?;

    // Compute fingerprint using ssh-keygen -lf
    let temp_dir = std::env::temp_dir();
    let temp_key_file = temp_dir.join(format!("nv-probe-{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temp_key_file, format!("{}\n", line))
        .map_err(|e| format!("Could not write temporary host key file: {e}"))?;

    let fp_output = Command::new("ssh-keygen")
        .args(["-lf", temp_key_file.to_str().unwrap()])
        .output()
        .await;

    let _ = fs::remove_file(&temp_key_file);

    let fp_raw = match fp_output {
        Ok(res) if res.status.success() => String::from_utf8_lossy(&res.stdout).to_string(),
        _ => "SHA256:Unknown".to_string(),
    };

    // Format of ssh-keygen -lf output: <bits> <fingerprint> <comment> (<type>)
    // e.g.: 256 SHA256:d8Q... [13.140.95.162]:22 (ED25519)
    let parts: Vec<&str> = fp_raw.split_whitespace().collect();
    let fingerprint = if parts.len() >= 2 {
        parts[1].to_string()
    } else {
        fp_raw.trim().to_string()
    };
    let key_type = if parts.len() >= 4 {
        parts[3].trim_matches(|c| c == '(' || c == ')').to_string()
    } else {
        "SSH-KEY".to_string()
    };

    Ok(HostKeyInfo {
        host: host.to_string(),
        port,
        key_type,
        fingerprint,
        raw_entry: line.to_string(),
    })
}

/// Atomically appends the user-approved host key to the application's known_hosts file.
pub fn save_approved_host_key(known_hosts_path: &Path, raw_entry: &str) -> Result<(), String> {
    if let Some(parent) = known_hosts_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create directory for known_hosts: {e}"))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(known_hosts_path)
        .map_err(|e| format!("Could not open application known_hosts file: {e}"))?;

    writeln!(file, "{}", raw_entry.trim())
        .map_err(|e| format!("Could not write to known_hosts file: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(known_hosts_path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}
