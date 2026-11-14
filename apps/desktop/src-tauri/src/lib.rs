pub mod keychain;
pub mod launcher;

use keychain::{delete_password, get_saved_password as keyring_get_password, save_password as keyring_save_password};
use launcher::launch_antigravity_preset;
use network_via_shell_ssh::{
    host_key::{probe_host_key as ssh_probe_host_key, save_approved_host_key, HostKeyInfo},
    SshProfile, SshTunnelManager, TunnelHealth,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedProfile {
    name: String,
    profile: SshProfile,
}

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create app config directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

fn profiles_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("profiles.json"))
}

fn known_hosts_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("known_hosts"))
}

fn resolve_askpass_path(app: &tauri::AppHandle) -> PathBuf {
    let exe_name = if cfg!(windows) { "nv-askpass.exe" } else { "nv-askpass" };

    if let Ok(res_dir) = app.path().resource_dir() {
        for candidate in [
            res_dir.join(exe_name),
            res_dir.join("resources").join(exe_name),
            res_dir.join("bin").join(exe_name),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    if let Ok(current) = std::env::current_exe() {
        if let Some(parent) = current.parent() {
            for candidate in [
                parent.join(exe_name),
                parent.join("resources").join(exe_name),
                parent.join("bin").join(exe_name),
            ] {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    PathBuf::from(exe_name)
}

fn read_profiles(app: &tauri::AppHandle) -> Result<Vec<SavedProfile>, String> {
    let path = profiles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Saved profiles are invalid: {e}"))
}

fn write_profiles(app: &tauri::AppHandle, profiles: &[SavedProfile]) -> Result<(), String> {
    let path = profiles_path(app)?;
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(profiles).map_err(|e| e.to_string())?;
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    fs::rename(&temp, &path).map_err(|e| format!("Could not save profiles atomically: {e}"))
}

#[tauri::command]
fn list_saved_profiles(app: tauri::AppHandle) -> Result<Vec<SavedProfile>, String> {
    read_profiles(&app)
}

#[tauri::command]
fn save_profile(app: tauri::AppHandle, name: String, profile: SshProfile) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 48 {
        return Err("Profile name must be 1–48 characters.".into());
    }
    profile.validate()?;
    let mut profiles = read_profiles(&app)?;
    if let Some(existing) = profiles
        .iter_mut()
        .find(|saved| saved.name.eq_ignore_ascii_case(name))
    {
        existing.profile = profile;
        existing.name = name.to_string();
    } else {
        if profiles.len() >= 50 {
            return Err("You can save up to 50 profiles.".into());
        }
        profiles.push(SavedProfile {
            name: name.to_string(),
            profile,
        });
    }
    write_profiles(&app, &profiles)
}

#[tauri::command]
fn delete_profile(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let mut profiles = read_profiles(&app)?;
    profiles.retain(|profile| profile.name != name);
    let _ = delete_password(&name);
    write_profiles(&app, &profiles)
}

#[tauri::command]
async fn probe_host(host: String, port: u16) -> Result<HostKeyInfo, String> {
    ssh_probe_host_key(&host, port).await
}

#[tauri::command]
fn approve_host(app: tauri::AppHandle, raw_entry: String) -> Result<(), String> {
    let path = known_hosts_path(&app)?;
    save_approved_host_key(&path, &raw_entry)
}

#[tauri::command]
async fn start_ssh_tunnel(
    app: tauri::AppHandle,
    profile: SshProfile,
    password: Option<String>,
    remember_password: bool,
    profile_name: Option<String>,
    manager: tauri::State<'_, SshTunnelManager>,
) -> Result<String, String> {
    let hosts_path = known_hosts_path(&app)?;
    let askpass_path = resolve_askpass_path(&app);

    // If remember_password is false and profile_name is given, delete any previously stored secret
    if let Some(ref name) = profile_name {
        if !remember_password {
            let _ = delete_password(name);
        }
    }

    let result = manager
        .start(profile, hosts_path, askpass_path, password.clone())
        .await
        .map_err(|e| e.to_string());

    // If connection succeeded and user asked to remember password, store in Windows Credential Manager
    if result.is_ok() {
        if let (Some(ref name), Some(ref pwd)) = (profile_name, password) {
            if remember_password {
                let _ = keyring_save_password(name, pwd);
            }
        }
    }

    result
}

#[tauri::command]
async fn stop_ssh_tunnel(manager: tauri::State<'_, SshTunnelManager>) -> Result<(), String> {
    manager.stop().await
}

#[tauri::command]
async fn get_tunnel_health(manager: tauri::State<'_, SshTunnelManager>) -> Result<TunnelHealth, String> {
    Ok(manager.get_health().await)
}

#[tauri::command]
fn get_saved_password(profile_name: String) -> Option<String> {
    keyring_get_password(&profile_name)
}

#[tauri::command]
fn launch_antigravity(socks_port: u16, custom_path: Option<String>) -> Result<(), String> {
    let path = custom_path.map(PathBuf::from);
    launch_antigravity_preset(socks_port, path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SshTunnelManager::default())
        .invoke_handler(tauri::generate_handler![
            start_ssh_tunnel,
            stop_ssh_tunnel,
            get_tunnel_health,
            list_saved_profiles,
            save_profile,
            delete_profile,
            probe_host,
            approve_host,
            get_saved_password,
            launch_antigravity
        ])
        .run(tauri::generate_context!())
        .expect("error while running NetworkViaShell");
}
