use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopStatus {
    app_version: String,
    platform: String,
    node_configured: bool,
    node_running: bool,
    rpc_reachable: bool,
}

#[tauri::command]
fn get_desktop_status(app: tauri::AppHandle) -> DesktopStatus {
    DesktopStatus {
        app_version: app.package_info().version.to_string(),
        platform: std::env::consts::OS.to_string(),
        node_configured: false,
        node_running: false,
        rpc_reachable: false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_desktop_status])
        .run(tauri::generate_context!())
        .expect("error while running PulseDAG Desktop");
}
