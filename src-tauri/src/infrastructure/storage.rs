use std::path::PathBuf;

use serde_json::json;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

use crate::domain::AppConfig;

const STORE_FILE: &str = "settings.json";
const CONFIG_KEY: &str = "config";

pub fn load_config(app_handle: &tauri::AppHandle) -> AppConfig {
    match app_handle.store(STORE_FILE) {
        Ok(store) => match store.get(CONFIG_KEY) {
            Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
            None => AppConfig::default(),
        },
        Err(e) => {
            tracing::warn!("Failed to load store: {}", e);
            AppConfig::default()
        }
    }
}

pub fn save_config(app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let store = app_handle.store(STORE_FILE).map_err(|e| e.to_string())?;
    let value = json!(config);
    store.set(CONFIG_KEY.to_string(), value);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_store_path(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .ok()
        .map(|p| p.join(STORE_FILE))
}
