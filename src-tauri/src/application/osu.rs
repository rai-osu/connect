use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use tokio::process::Command as TokioCommand;

use crate::domain::AppConfig;

#[cfg(target_os = "windows")]
mod deelevate {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        DuplicateTokenEx, SecurityImpersonation, TokenPrimary, TOKEN_ACCESS_MASK,
        TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{
        CreateProcessWithTokenW, OpenProcess, OpenProcessToken, LOGON_WITH_PROFILE,
        PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    struct HandleGuard(HANDLE);

    impl Drop for HandleGuard {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
    }

    /// Launches with medium integrity by borrowing explorer.exe's token.
    pub fn launch_deelevated(
        exe_path: &Path,
        args: &[&str],
        working_dir: &Path,
    ) -> Result<(), String> {
        unsafe {
            let shell_window = GetShellWindow();
            if shell_window.0.is_null() {
                return Err("Failed to get shell window - explorer.exe may not be running".into());
            }

            let mut shell_pid: u32 = 0;
            GetWindowThreadProcessId(shell_window, Some(&mut shell_pid));
            if shell_pid == 0 {
                return Err("Failed to get shell process ID".into());
            }

            let shell_process = HandleGuard(
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, shell_pid)
                    .map_err(|e| format!("Failed to open shell process: {}", e))?,
            );

            let mut shell_token = HANDLE::default();
            OpenProcessToken(shell_process.0, TOKEN_DUPLICATE, &mut shell_token)
                .map_err(|e| format!("Failed to open shell process token: {}", e))?;
            let shell_token = HandleGuard(shell_token);

            let mut primary_token = HANDLE::default();
            let desired_access =
                TOKEN_ACCESS_MASK(TOKEN_QUERY.0 | TOKEN_DUPLICATE.0 | TOKEN_ASSIGN_PRIMARY.0);
            DuplicateTokenEx(
                shell_token.0,
                desired_access,
                None,
                SecurityImpersonation,
                TokenPrimary,
                &mut primary_token,
            )
            .map_err(|e| format!("Failed to duplicate token: {}", e))?;
            let primary_token = HandleGuard(primary_token);

            let exe_str = exe_path.to_string_lossy();
            let cmd_line = if args.is_empty() {
                format!("\"{}\"", exe_str)
            } else {
                format!("\"{}\" {}", exe_str, args.join(" "))
            };
            let mut cmd_wide = to_wide(&cmd_line);
            let working_dir_wide = to_wide(&working_dir.to_string_lossy());

            let mut startup_info = STARTUPINFOW::default();
            startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let mut process_info = PROCESS_INFORMATION::default();

            CreateProcessWithTokenW(
                primary_token.0,
                LOGON_WITH_PROFILE,
                None,
                windows::core::PWSTR(cmd_wide.as_mut_ptr()),
                windows::Win32::System::Threading::PROCESS_CREATION_FLAGS(0),
                None,
                windows::core::PCWSTR(working_dir_wide.as_ptr()),
                &startup_info,
                &mut process_info,
            )
            .map_err(|e| format!("Failed to create de-elevated process: {}", e))?;

            let _ = CloseHandle(process_info.hProcess);
            let _ = CloseHandle(process_info.hThread);

            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
use deelevate::launch_deelevated;

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

    #[cfg(target_os = "windows")]
    {
        match launch_deelevated(&exe_path, &["-devserver", devserver_host], osu_path) {
            Ok(()) => return Ok(()),
            Err(e) => tracing::warn!("De-elevated launch failed ({}), using fallback", e),
        }

        let result = Command::new(&exe_path)
            .arg("-devserver")
            .arg(devserver_host)
            .current_dir(osu_path)
            .spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to launch osu!: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let result = Command::new(&exe_path)
            .arg("-devserver")
            .arg(devserver_host)
            .current_dir(osu_path)
            .spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to launch osu!: {}", e)),
        }
    }
}

#[cfg(target_os = "windows")]
pub async fn is_osu_running() -> bool {
    let output = TokioCommand::new("tasklist")
        .args(["/FI", "IMAGENAME eq osu!.exe", "/NH"])
        .output()
        .await;

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("osu!.exe")
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn is_osu_running() -> bool {
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
