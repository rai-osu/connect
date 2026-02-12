import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, AppState } from "$lib/types";
import { defaultConfig, defaultState } from "$lib/types";

// Use object wrappers to allow property mutation instead of reassignment
export const store = $state({
  config: defaultConfig as AppConfig,
  appState: defaultState as AppState,
  isLoading: false
});

// Convenience exports for direct access
export const config = $derived(store.config);
export const appState = $derived(store.appState);
export const isLoading = $derived(store.isLoading);

// Derived state
export const isConnected = $derived(store.appState.status === "connected");
export const isConnecting = $derived(store.appState.status === "connecting");
export const hasError = $derived(store.appState.status === "error");
export const canConnect = $derived(
  !isConnected && !isConnecting && store.config.osu_path !== null
);

export async function loadConfig(): Promise<void> {
  try {
    const savedConfig = await invoke<AppConfig>("load_saved_config");
    store.config = savedConfig;
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

export async function saveConfig(newConfig: AppConfig): Promise<void> {
  try {
    await invoke("set_config", { config: newConfig });
    store.config = newConfig;
  } catch (e) {
    console.error("Failed to save config:", e);
    throw e;
  }
}

export async function updateConfig<K extends keyof AppConfig>(
  key: K,
  value: AppConfig[K]
): Promise<void> {
  const newConfig = { ...store.config, [key]: value };
  await saveConfig(newConfig);
}

export async function detectOsuPath(): Promise<string | null> {
  try {
    return await invoke<string | null>("detect_osu");
  } catch (e) {
    console.error("Failed to detect osu! path:", e);
    return null;
  }
}

export async function validateOsuPath(path: string): Promise<boolean> {
  try {
    return await invoke<boolean>("validate_osu_path", { path });
  } catch (e) {
    console.error("Failed to validate osu! path:", e);
    return false;
  }
}

export async function connect(): Promise<void> {
  if (!canConnect) return;

  store.isLoading = true;
  store.appState.status = "connecting";
  store.appState.last_error = null;

  try {
    await invoke("connect");
    store.appState.status = "connected";
  } catch (e) {
    console.error("Failed to connect:", e);
    store.appState.status = "error";
    store.appState.last_error = String(e);
  } finally {
    store.isLoading = false;
  }
}

export async function disconnect(): Promise<void> {
  if (!isConnected) return;

  store.isLoading = true;

  try {
    await invoke("disconnect");
    store.appState.status = "disconnected";
    store.appState.requests_proxied = 0;
    store.appState.beatmaps_downloaded = 0;
  } catch (e) {
    console.error("Failed to disconnect:", e);
    store.appState.last_error = String(e);
  } finally {
    store.isLoading = false;
  }
}

export async function refreshStatus(): Promise<void> {
  try {
    const state = await invoke<AppState>("get_status");
    store.appState = state;
  } catch (e) {
    console.error("Failed to refresh status:", e);
  }
}

export async function checkOsuRunning(): Promise<boolean> {
  try {
    const running = await invoke<boolean>("is_osu_running_cmd");
    store.appState.osu_running = running;
    return running;
  } catch (e) {
    console.error("Failed to check osu! status:", e);
    return false;
  }
}
