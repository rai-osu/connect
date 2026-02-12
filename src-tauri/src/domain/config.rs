use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub osu_path: Option<PathBuf>,
    pub start_at_boot: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub debug_logging: bool,
    pub proxy: ProxyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osu_path: None,
            start_at_boot: false,
            minimize_to_tray: true,
            start_minimized: false,
            debug_logging: false,
            proxy: ProxyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub http_port: u16,
    /// Inject supporter privileges into Bancho responses.
    /// When enabled, modifies UserPrivileges packets in HTTP responses from c.ppy.sh
    /// to include supporter status, enabling osu!direct in the client.
    pub inject_supporter: bool,
    pub api_base_url: String,
    pub direct_base_url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            http_port: 80,
            inject_supporter: false,
            api_base_url: "https://api.rai.moe".to_string(),
            direct_base_url: "https://direct.rai.moe".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub status: ConnectionStatus,
    pub osu_running: bool,
    pub requests_proxied: u64,
    pub beatmaps_downloaded: u64,
    pub last_error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            osu_running: false,
            requests_proxied: 0,
            beatmaps_downloaded: 0,
            last_error: None,
        }
    }
}
