use std::path::PathBuf;

use parking_lot::RwLock;
use tauri::{AppHandle, Manager, State};

use crate::application::{
    detect_osu_path, get_osu_path, is_osu_running, is_valid_osu_installation, launch_osu,
    ProxyManager,
};
use crate::domain::{AppConfig, AppState, ConnectionStatus};
use crate::infrastructure::storage::{load_config, save_config};

pub struct TauriState {
    pub config: RwLock<AppConfig>,
    pub proxy: RwLock<Option<ProxyManager>>,
}

impl Default for TauriState {
    fn default() -> Self {
        Self {
            config: RwLock::new(AppConfig::default()),
            proxy: RwLock::new(None),
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
pub fn is_osu_running_cmd() -> bool {
    is_osu_running()
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
    let mut proxy = state.proxy.write();

    if let Some(ref mut pm) = *proxy {
        pm.stop().await?;
    }

    *proxy = None;

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
