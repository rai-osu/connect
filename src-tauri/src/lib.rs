pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use infrastructure::logging::{LogBuffer, LogCaptureLayer};
use interface::{
    clear_logs, connect, detect_osu, disconnect, get_config, get_logs, get_status, hide_window,
    is_osu_running_cmd, load_saved_config, quit_app, set_config, show_window, validate_osu_path,
    TauriState,
};

fn init_logging(log_buffer: LogBuffer) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rai_connect=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(LogCaptureLayer::new(log_buffer))
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create log buffer before initializing tracing so we capture boot logs
    let log_buffer = LogBuffer::new();
    init_logging(log_buffer.clone());

    tracing::info!("Starting rai!connect v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app| {
            let state = TauriState::new(log_buffer);
            let config = infrastructure::storage::load_config(app.handle());
            *state.config.write() = config.clone();
            app.manage(state);
            setup_tray(app)?;
            tracing::info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            load_saved_config,
            detect_osu,
            validate_osu_path,
            is_osu_running_cmd,
            get_status,
            connect,
            disconnect,
            hide_window,
            show_window,
            quit_app,
            get_logs,
            clear_logs,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<TauriState>();
                let config = state.config.read();
                if config.minimize_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                let state = app_handle.state::<TauriState>();
                let proxy = state.proxy.read();
                if proxy.is_some() {
                    api.prevent_exit();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
        });
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("rai!connect")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
