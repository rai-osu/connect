use std::path::PathBuf;

use crate::domain::AppConfig;

const CONFIG_KEY: &str = "config";

pub fn load_config(app_handle: &tauri::AppHandle) -> AppConfig {
    match app_handle.try_state::<tauri_plugin_store::Store<tauri::Wry>>() {
        Some(store) => match store.get(CONFIG_KEY) {
            Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
            None => AppConfig::default(),
        },
        None => {
            tracing::warn!("Store not available, using default config");
            AppConfig::default()
        }
    }
}

pub fn save_config(app_handle: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    match app_handle.try_state::<tauri_plugin_store::Store<tauri::Wry>>() {
        Some(store) => {
            let value = serde_json::to_value(config).map_err(|e| e.to_string())?;
            store.set(CONFIG_KEY.to_string(), value);
            store.save().map_err(|e| e.to_string())?;
            Ok(())
        }
        None => Err("Store not available".to_string()),
    }
}

pub fn get_store_path(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .ok()
        .map(|p| p.join("settings.json"))
}
