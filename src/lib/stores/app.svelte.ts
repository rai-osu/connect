/**
 * Application state using Svelte 5 runes
 *
 * This file uses the .svelte.ts extension to enable runes outside components.
 * State is exported directly for cross-component reactivity.
 */

import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, AppState } from "$lib/types";
import { defaultConfig, defaultState } from "$lib/types";

// ============================================================================
// Reactive State (exported for cross-component reactivity)
// ============================================================================

export let config = $state<AppConfig>(defaultConfig);
export let appState = $state<AppState>(defaultState);
export let isLoading = $state(false);

// ============================================================================
// Derived State
// ============================================================================

export const isConnected = $derived(appState.status === "connected");
export const isConnecting = $derived(appState.status === "connecting");
export const hasError = $derived(appState.status === "error");
export const canConnect = $derived(
  !isConnected && !isConnecting && config.osu_path !== null
);

// ============================================================================
// Actions
// ============================================================================

/**
 * Loads the saved configuration from the Rust backend
 */
export async function loadConfig(): Promise<void> {
  try {
    const savedConfig = await invoke<AppConfig>("load_saved_config");
    config = savedConfig;
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

/**
 * Saves the current configuration to the Rust backend
 */
export async function saveConfig(newConfig: AppConfig): Promise<void> {
  try {
    await invoke("set_config", { config: newConfig });
    config = newConfig;
  } catch (e) {
    console.error("Failed to save config:", e);
    throw e;
  }
}

/**
 * Updates a specific config field
 */
export async function updateConfig<K extends keyof AppConfig>(
  key: K,
  value: AppConfig[K]
): Promise<void> {
  const newConfig = { ...config, [key]: value };
  await saveConfig(newConfig);
}

/**
 * Detects the osu! installation path
 */
export async function detectOsuPath(): Promise<string | null> {
  try {
    return await invoke<string | null>("detect_osu");
  } catch (e) {
    console.error("Failed to detect osu! path:", e);
    return null;
  }
}

/**
 * Validates a given path as an osu! installation
 */
export async function validateOsuPath(path: string): Promise<boolean> {
  try {
    return await invoke<boolean>("validate_osu_path", { path });
  } catch (e) {
    console.error("Failed to validate osu! path:", e);
    return false;
  }
}

/**
 * Connects to the proxy and launches osu!
 */
export async function connect(): Promise<void> {
  if (!canConnect) return;

  isLoading = true;
  appState.status = "connecting";
  appState.last_error = null;

  try {
    await invoke("connect");
    appState.status = "connected";
  } catch (e) {
    console.error("Failed to connect:", e);
    appState.status = "error";
    appState.last_error = String(e);
  } finally {
    isLoading = false;
  }
}

/**
 * Disconnects from the proxy
 */
export async function disconnect(): Promise<void> {
  if (!isConnected) return;

  isLoading = true;

  try {
    await invoke("disconnect");
    appState.status = "disconnected";
    appState.requests_proxied = 0;
    appState.beatmaps_downloaded = 0;
  } catch (e) {
    console.error("Failed to disconnect:", e);
    appState.last_error = String(e);
  } finally {
    isLoading = false;
  }
}

/**
 * Refreshes the current status from the backend
 */
export async function refreshStatus(): Promise<void> {
  try {
    const state = await invoke<AppState>("get_status");
    appState = state;
  } catch (e) {
    console.error("Failed to refresh status:", e);
  }
}

/**
 * Checks if osu! is currently running
 */
export async function checkOsuRunning(): Promise<boolean> {
  try {
    const running = await invoke<boolean>("is_osu_running_cmd");
    appState.osu_running = running;
    return running;
  } catch (e) {
    console.error("Failed to check osu! status:", e);
    return false;
  }
}
