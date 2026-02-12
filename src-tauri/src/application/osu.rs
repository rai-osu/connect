use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::AppConfig;

const OSU_COMMON_PATHS: &[&str] = &[
    r"%LOCALAPPDATA%\osu!",
    r"%APPDATA%\osu!",
    r"%USERPROFILE%\AppData\Local\osu!",
    r"C:\osu!",
    r"C:\Games\osu!",
    r"D:\osu!",
    r"D:\Games\osu!",
];

pub fn detect_osu_path() -> Option<PathBuf> {
    for path_template in OSU_COMMON_PATHS {
        let expanded = expand_env_vars(path_template);
        let path = PathBuf::from(&expanded);

        if is_valid_osu_installation(&path) {
            return Some(path);
        }
    }

    None
}

pub fn is_valid_osu_installation(path: &Path) -> bool {
    let exe_path = path.join("osu!.exe");
    exe_path.exists() && exe_path.is_file()
}

fn expand_env_vars(path: &str) -> String {
    let mut result = path.to_string();

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        result = result.replace("%LOCALAPPDATA%", &local_app_data);
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        result = result.replace("%APPDATA%", &app_data);
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        result = result.replace("%USERPROFILE%", &user_profile);
    }

    result
}

pub fn launch_osu(osu_path: &Path, devserver_host: &str) -> Result<(), String> {
    let exe_path = osu_path.join("osu!.exe");

    if !exe_path.exists() {
        return Err(format!("osu!.exe not found at {:?}", exe_path));
    }

    let result = Command::new(&exe_path)
        .arg(format!("-devserver {}", devserver_host))
        .current_dir(osu_path)
        .spawn();

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to launch osu!: {}", e)),
    }
}

#[cfg(target_os = "windows")]
pub fn is_osu_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq osu!.exe", "/NH"])
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("osu!.exe")
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_osu_running() -> bool {
    false
}

pub fn get_osu_path(config: &AppConfig) -> Option<PathBuf> {
    if let Some(ref path) = config.osu_path {
        if is_valid_osu_installation(path) {
            return Some(path.clone());
        }
    }

    detect_osu_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        let path = r"%USERPROFILE%\test";
        let expanded = expand_env_vars(path);
        assert!(!expanded.contains("%USERPROFILE%") || expanded == path);
    }
}
