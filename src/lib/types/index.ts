/**
 * TypeScript types matching the Rust domain types
 */

export type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

export interface ProxyConfig {
  http_port: number;
  inject_supporter: boolean;
  api_base_url: string;
  direct_base_url: string;
}

export interface AppConfig {
  osu_path: string | null;
  start_at_boot: boolean;
  minimize_to_tray: boolean;
  start_minimized: boolean;
  debug_logging: boolean;
  proxy: ProxyConfig;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export interface AppState {
  status: ConnectionStatus;
  osu_running: boolean;
  requests_proxied: number;
  beatmaps_downloaded: number;
  last_error: string | null;
}

export const defaultConfig: AppConfig = {
  osu_path: null,
  start_at_boot: false,
  minimize_to_tray: true,
  start_minimized: false,
  debug_logging: false,
  proxy: {
    http_port: 80,
    inject_supporter: false,
    api_base_url: "https://api.rai.moe",
    direct_base_url: "https://direct.rai.moe",
  },
};

export const defaultState: AppState = {
  status: "disconnected",
  osu_running: false,
  requests_proxied: 0,
  beatmaps_downloaded: 0,
  last_error: null,
};
