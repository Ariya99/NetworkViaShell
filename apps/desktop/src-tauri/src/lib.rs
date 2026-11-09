use network_via_shell_ssh::{SshProfile, SshTunnelManager};

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
        .invoke_handler(tauri::generate_handler![start_ssh_tunnel, stop_ssh_tunnel])
        .run(tauri::generate_context!())
        .expect("error while running NetworkViaShell");
}
