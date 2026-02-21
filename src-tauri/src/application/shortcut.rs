use std::path::PathBuf;

pub fn create_desktop_shortcut() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        create_windows_shortcut()
    }

    #[cfg(target_os = "linux")]
    {
        create_linux_shortcut()
    }

    #[cfg(target_os = "macos")]
    {
        Err("Desktop shortcuts are not yet supported on macOS".to_string())
    }
}

pub fn shortcut_exists() -> bool {
    get_shortcut_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn remove_desktop_shortcut() -> Result<(), String> {
    let path = get_shortcut_path().ok_or("Could not determine shortcut path")?;

    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to remove shortcut: {}", e))?;
    }

    Ok(())
}

fn get_shortcut_path() -> Option<PathBuf> {
    let desktop = dirs::desktop_dir()?;

    #[cfg(target_os = "windows")]
    {
        Some(desktop.join("Launch osu! with rai.lnk"))
    }

    #[cfg(target_os = "linux")]
    {
        Some(desktop.join("launch-osu-with-rai.desktop"))
    }

    #[cfg(target_os = "macos")]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn create_windows_shortcut() -> Result<PathBuf, String> {
    use mslnk::ShellLink;

    let desktop = dirs::desktop_dir().ok_or("Could not find desktop directory")?;
    let shortcut_path = desktop.join("Launch osu! with rai.lnk");

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;

    let icon_path = exe_path.clone();

    let mut sl =
        ShellLink::new(&exe_path).map_err(|e| format!("Failed to create shortcut: {}", e))?;

    sl.set_arguments(Some("--launch-osu".to_string()));
    sl.set_icon_location(Some(icon_path.to_string_lossy().to_string()));
    sl.set_working_dir(exe_path.parent().map(|p| p.to_string_lossy().to_string()));

    sl.create_lnk(&shortcut_path)
        .map_err(|e| format!("Failed to save shortcut: {}", e))?;

    tracing::info!("Created desktop shortcut at {:?}", shortcut_path);
    Ok(shortcut_path)
}

#[cfg(target_os = "linux")]
fn create_linux_shortcut() -> Result<PathBuf, String> {
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let desktop = dirs::desktop_dir().ok_or("Could not find desktop directory")?;
    let shortcut_path = desktop.join("launch-osu-with-rai.desktop");

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;

    let icon_path = find_linux_icon(&exe_path);

    let desktop_entry = format!(
        r#"[Desktop Entry]
Type=Application
Name=Launch osu! with rai
Comment=Start rai!connect proxy and launch osu!
Exec="{}" --launch-osu
Icon={}
Terminal=false
Categories=Game;
"#,
        exe_path.display(),
        icon_path.display()
    );

    let mut file = File::create(&shortcut_path)
        .map_err(|e| format!("Failed to create desktop file: {}", e))?;

    file.write_all(desktop_entry.as_bytes())
        .map_err(|e| format!("Failed to write desktop file: {}", e))?;

    let mut perms = fs::metadata(&shortcut_path)
        .map_err(|e| format!("Failed to get file permissions: {}", e))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shortcut_path, perms)
        .map_err(|e| format!("Failed to set file permissions: {}", e))?;

    tracing::info!("Created desktop shortcut at {:?}", shortcut_path);
    Ok(shortcut_path)
}

#[cfg(target_os = "linux")]
fn find_linux_icon(exe_path: &std::path::Path) -> PathBuf {
    if let Some(parent) = exe_path.parent() {
        let candidates = [
            parent.join("icons/icon.png"),
            parent.join("icon.png"),
            parent.join("../share/icons/hicolor/256x256/apps/rai-connect.png"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return candidate.clone();
            }
        }
    }

    PathBuf::from("rai-connect")
}
