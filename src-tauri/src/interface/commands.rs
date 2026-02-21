use std::path::PathBuf;

use parking_lot::RwLock;
use tauri::{tray::TrayIconId, AppHandle, Manager, State};

use crate::application::{
    detect_osu_path, get_osu_path, is_osu_running, is_valid_osu_installation, launch_osu,
    ProxyManager,
};
use crate::domain::{AppConfig, AppState};
use crate::infrastructure::logging::{LogBuffer, LogEntry};
use crate::infrastructure::storage::{load_config, save_config};
use crate::infrastructure::tls;

pub struct TauriState {
    pub config: RwLock<AppConfig>,
    pub proxy: RwLock<Option<ProxyManager>>,
    pub logs: LogBuffer,
}

impl TauriState {
    pub fn new(logs: LogBuffer) -> Self {
        Self {
            config: RwLock::new(AppConfig::default()),
            proxy: RwLock::new(None),
            logs,
        }
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, TauriState>) -> AppConfig {
    state.config.read().clone()
}

#[tauri::command]
pub fn set_config(
    app: AppHandle,
    state: State<'_, TauriState>,
    config: AppConfig,
) -> Result<(), String> {
    *state.config.write() = config.clone();
    save_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub fn load_saved_config(app: AppHandle, state: State<'_, TauriState>) -> AppConfig {
    let config = load_config(&app);
    *state.config.write() = config.clone();
    config
}

#[tauri::command]
pub fn detect_osu() -> Option<PathBuf> {
    detect_osu_path()
}

#[tauri::command]
pub fn validate_osu_path(path: String) -> bool {
    is_valid_osu_installation(&PathBuf::from(path))
}

#[tauri::command]
pub async fn is_osu_running_cmd() -> bool {
    is_osu_running().await
}

#[tauri::command]
pub fn get_status(state: State<'_, TauriState>) -> AppState {
    let proxy = state.proxy.read();
    match proxy.as_ref() {
        Some(pm) => pm.state().read().clone(),
        None => AppState::default(),
    }
}

#[tauri::command]
pub async fn start_proxy(state: State<'_, TauriState>) -> Result<(), String> {
    let config = state.config.read().clone();

    let mut proxy_manager = ProxyManager::new(config.proxy.clone());
    proxy_manager.start().await?;
    *state.proxy.write() = Some(proxy_manager);

    Ok(())
}

#[tauri::command]
pub async fn connect(state: State<'_, TauriState>) -> Result<(), String> {
    let config = state.config.read().clone();
    let osu_path = get_osu_path(&config)
        .ok_or("osu! installation not found. Please configure the path in settings.")?;

    let mut proxy_manager = ProxyManager::new(config.proxy.clone());
    proxy_manager.start().await?;
    *state.proxy.write() = Some(proxy_manager);

    launch_osu(&osu_path, "localhost")?;
    Ok(())
}

#[tauri::command]
pub async fn disconnect(state: State<'_, TauriState>) -> Result<(), String> {
    let pm = state.proxy.write().take();

    if let Some(mut pm) = pm {
        pm.stop().await?;
    }

    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn show_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn get_logs(state: State<'_, TauriState>, count: Option<usize>) -> Vec<LogEntry> {
    match count {
        Some(n) => state.logs.get_recent(n),
        None => state.logs.get_all(),
    }
}

/// Get only logs newer than the given ID for differential updates.
/// This is much more efficient than get_logs() when polling frequently,
/// as it only returns new entries since the last fetch.
#[tauri::command]
pub fn get_logs_since(state: State<'_, TauriState>, last_id: u64) -> Vec<LogEntry> {
    state.logs.get_logs_since(last_id)
}

/// Get the ID of the most recent log entry.
/// Returns 0 if the log buffer is empty.
/// The frontend can use this to initialize its tracking state.
#[tauri::command]
pub fn get_latest_log_id(state: State<'_, TauriState>) -> u64 {
    state.logs.get_latest_id()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, TauriState>) {
    state.logs.clear();
}

#[tauri::command]
pub fn is_certificate_installed() -> bool {
    tls::is_certificate_installed()
}

#[tauri::command]
pub fn install_certificate() -> Result<bool, String> {
    tls::install_certificate().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_certificate_path() -> Result<String, String> {
    tls::get_cert_path()
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

/// Update the system tray tooltip to reflect the current connection status.
/// Called by the frontend when the connection status changes.
#[tauri::command]
pub fn update_tray_status(app: AppHandle, status: String, downloads: Option<u64>) {
    let tray_id: TrayIconId = "main".into();
    if let Some(tray) = app.tray_by_id(&tray_id) {
        let tooltip = match (status.as_str(), downloads) {
            ("connected", Some(count)) if count > 0 => {
                format!("rai!connect - Connected ({} downloads)", count)
            }
            ("connected", _) => "rai!connect - Connected".to_string(),
            ("connecting", _) => "rai!connect - Connecting...".to_string(),
            ("disconnected", _) => "rai!connect - Disconnected".to_string(),
            ("error", _) => "rai!connect - Error".to_string(),
            _ => format!("rai!connect - {}", status),
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}
