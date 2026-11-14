use network_via_shell_ssh::{SshProfile, SshTunnelManager};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::{Path, PathBuf}};
use tauri::Manager;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedProfile {
    name: String,
    profile: SshProfile,
}

fn profiles_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create app config directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(dir.join("profiles.json"))
}

fn read_profiles(app: &tauri::AppHandle) -> Result<Vec<SavedProfile>, String> {
    let path = profiles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Saved profiles are invalid: {e}"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Profile file has no parent directory.".to_string())?;
    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|e| format!("Could not create temporary profile file: {e}"))?;
    temp.write_all(bytes)
        .map_err(|e| format!("Could not write temporary profile file: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    temp.persist(path)
        .map_err(|e| format!("Could not save profiles atomically: {}", e.error))?;
    Ok(())
}

fn write_profiles(app: &tauri::AppHandle, profiles: &[SavedProfile]) -> Result<(), String> {
    let path = profiles_path(app)?;
    let bytes = serde_json::to_vec_pretty(profiles).map_err(|e| e.to_string())?;
    atomic_write(&path, &bytes)
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
    write_profiles(&app, &profiles)
}

#[tauri::command]
async fn start_ssh_tunnel(
    profile: SshProfile,
    manager: tauri::State<'_, SshTunnelManager>,
) -> Result<String, String> {
    manager.start(profile).await
}

#[tauri::command]
async fn stop_ssh_tunnel(manager: tauri::State<'_, SshTunnelManager>) -> Result<(), String> {
    manager.stop().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SshTunnelManager::default())
        .invoke_handler(tauri::generate_handler![
            start_ssh_tunnel,
            stop_ssh_tunnel,
            list_saved_profiles,
            save_profile,
            delete_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running NetworkViaShell");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_an_existing_profile_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("profiles.json");

        fs::write(&path, b"old profile data").expect("write initial profile data");
        atomic_write(&path, b"updated profile data").expect("replace existing profile file");

        assert_eq!(
            fs::read(&path).expect("read updated profile data"),
            b"updated profile data"
        );
    }
}
