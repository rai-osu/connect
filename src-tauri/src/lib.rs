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

use application::{get_osu_path, launch_osu, ProxyManager};
use infrastructure::logging::{LogBuffer, LogCaptureLayer};
use interface::{
    check_shortcut_exists, clear_logs, connect, create_launch_shortcut, detect_osu, disconnect,
    get_certificate_path, get_config, get_latest_log_id, get_logs, get_logs_since, get_status,
    hide_window, install_certificate, is_certificate_installed, is_osu_running_cmd,
    load_saved_config, quit_app, remove_launch_shortcut, set_config, show_window, start_proxy,
    update_tray_status, validate_osu_path, TauriState,
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
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let has_launch_osu = args.iter().any(|a| a == "--launch-osu");

            if has_launch_osu {
                tracing::info!("Second instance with --launch-osu, triggering osu! launch");
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<TauriState>();
                    let config = state.config.read().clone();

                    // Check if proxy is already running
                    let proxy_running = state.proxy.read().is_some();

                    if !proxy_running {
                        let mut proxy_manager = ProxyManager::new(config.proxy.clone());
                        if let Err(e) = proxy_manager.start().await {
                            tracing::error!("--launch-osu: Failed to start proxy: {}", e);
                            return;
                        }
                        *state.proxy.write() = Some(proxy_manager);
                        tracing::info!("--launch-osu: Proxy started");
                    } else {
                        tracing::info!("--launch-osu: Proxy already running");
                    }

                    // Launch osu!
                    if let Some(osu_path) = get_osu_path(&config) {
                        if let Err(e) = launch_osu(&osu_path, "localhost") {
                            tracing::error!("--launch-osu: Failed to launch osu!: {}", e);
                        } else {
                            tracing::info!("--launch-osu: osu! launched successfully");
                        }
                    } else {
                        tracing::error!("--launch-osu: osu! path not configured");
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            } else {
                // Regular second instance: just focus the window
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    tracing::info!("Second instance detected, focusing existing window");
                }
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app| {
            let state = TauriState::new(log_buffer);
            let config = infrastructure::storage::load_config(app.handle());
            *state.config.write() = config.clone();
            app.manage(state);
            setup_tray(app)?;

            let has_minimized_flag = std::env::args().any(|a| a == "--minimized");
            let has_launch_osu_flag = std::env::args().any(|a| a == "--launch-osu");

            if has_launch_osu_flag {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }

                let app_handle = app.handle().clone();
                let config_clone = config.clone();

                tauri::async_runtime::spawn(async move {
                    tracing::info!("--launch-osu: Starting proxy and launching osu!");

                    let mut proxy_manager = ProxyManager::new(config_clone.proxy.clone());
                    if let Err(e) = proxy_manager.start().await {
                        tracing::error!("--launch-osu: Failed to start proxy: {}", e);
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        return;
                    }

                    let state = app_handle.state::<TauriState>();
                    *state.proxy.write() = Some(proxy_manager);

                    if let Some(osu_path) = get_osu_path(&config_clone) {
                        if let Err(e) = launch_osu(&osu_path, "localhost") {
                            tracing::error!("--launch-osu: Failed to launch osu!: {}", e);
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        } else {
                            tracing::info!("--launch-osu: osu! launched successfully");
                        }
                    } else {
                        tracing::error!("--launch-osu: osu! path not configured");
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            } else if config.start_minimized || has_minimized_flag {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

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
            start_proxy,
            connect,
            disconnect,
            hide_window,
            show_window,
            quit_app,
            get_logs,
            get_logs_since,
            get_latest_log_id,
            clear_logs,
            is_certificate_installed,
            install_certificate,
            get_certificate_path,
            update_tray_status,
            create_launch_shortcut,
            check_shortcut_exists,
            remove_launch_shortcut,
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

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("rai!connect - Disconnected")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                let state = app.state::<TauriState>();
                let proxy = state.proxy.write().take();
                if let Some(mut pm) = proxy {
                    tauri::async_runtime::spawn(async move {
                        let _ = pm.stop().await;
                    });
                }
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
