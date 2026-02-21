import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, AppState, LogEntry } from "$lib/types";
import { defaultConfig, defaultState } from "$lib/types";

const loadingOperations = $state(new Set<string>());

export const store = $state({
  config: defaultConfig as AppConfig,
  appState: defaultState as AppState,
  logs: [] as LogEntry[],
  get isLoading(): boolean {
    return loadingOperations.size > 0;
  }
});

function startLoading(operation: string): void {
  loadingOperations.add(operation);
}

function stopLoading(operation: string): void {
  loadingOperations.delete(operation);
}

function setError(operation: string, error: unknown): void {
  let message: string;
  if (error instanceof Error) {
    message = error.message;
  } else if (typeof error === "string") {
    message = error;
  } else if (error && typeof error === "object" && "message" in error) {
    message = String((error as { message: unknown }).message);
  } else {
    message = String(error);
  }
  store.appState.last_error = `Failed to ${operation}: ${message}`;
}

async function updateTrayStatus(status: string, downloads?: number): Promise<void> {
  try {
    await invoke("update_tray_status", { status, downloads: downloads ?? null });
  } catch {
    // Not critical
  }
}
export function isConnected(): boolean {
  return store.appState.status === "connected";
}

export function isConnecting(): boolean {
  return store.appState.status === "connecting";
}

export function hasError(): boolean {
  return store.appState.status === "error";
}

export function canConnect(): boolean {
  return !isConnected() && !isConnecting() && store.config.osu_path !== null;
}

export async function loadConfig(): Promise<void> {
  startLoading("loadConfig");
  try {
    const savedConfig = await invoke<AppConfig>("load_saved_config");
    store.config = savedConfig;
  } catch (e) {
    console.error("Failed to load config:", e);
    setError("load config", e);
  } finally {
    stopLoading("loadConfig");
  }
}

export async function saveConfig(newConfig: AppConfig): Promise<void> {
  startLoading("saveConfig");
  try {
    await invoke("set_config", { config: newConfig });
    store.config = newConfig;
  } catch (e) {
    console.error("Failed to save config:", e);
    setError("save config", e);
    throw e;
  } finally {
    stopLoading("saveConfig");
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
  startLoading("detectOsuPath");
  try {
    return await invoke<string | null>("detect_osu");
  } catch (e) {
    console.error("Failed to detect osu! path:", e);
    setError("detect osu! path", e);
    return null;
  } finally {
    stopLoading("detectOsuPath");
  }
}

export async function validateOsuPath(path: string): Promise<boolean> {
  startLoading("validateOsuPath");
  try {
    return await invoke<boolean>("validate_osu_path", { path });
  } catch (e) {
    console.error("Failed to validate osu! path:", e);
    setError("validate osu! path", e);
    return false;
  } finally {
    stopLoading("validateOsuPath");
  }
}

export async function startProxy(): Promise<void> {
  if (isConnected() || isConnecting()) return;

  startLoading("startProxy");
  store.appState.status = "connecting";
  store.appState.last_error = null;
  updateTrayStatus("connecting");

  try {
    await invoke("start_proxy");
    store.appState.status = "connected";
    updateTrayStatus("connected");
  } catch (e) {
    console.error("Failed to start proxy:", e);
    store.appState.status = "error";
    setError("start proxy", e);
    updateTrayStatus("error");
  } finally {
    stopLoading("startProxy");
  }
}

export async function connect(): Promise<void> {
  if (!canConnect()) return;

  startLoading("connect");
  store.appState.status = "connecting";
  store.appState.last_error = null;
  updateTrayStatus("connecting");

  try {
    await invoke("connect");
    store.appState.status = "connected";
    updateTrayStatus("connected");
  } catch (e) {
    console.error("Failed to connect:", e);
    store.appState.status = "error";
    setError("connect", e);
    updateTrayStatus("error");
  } finally {
    stopLoading("connect");
  }
}

export async function disconnect(): Promise<void> {
  if (!isConnected()) return;

  startLoading("disconnect");

  try {
    await invoke("disconnect");
    store.appState.status = "disconnected";
    store.appState.requests_proxied = 0;
    store.appState.beatmaps_downloaded = 0;
    updateTrayStatus("disconnected");
  } catch (e) {
    console.error("Failed to disconnect:", e);
    setError("disconnect", e);
  } finally {
    stopLoading("disconnect");
  }
}

export async function refreshStatus(): Promise<void> {
  try {
    const newState = await invoke<AppState>("get_status");

    const statusChanged = store.appState.status !== newState.status;
    const downloadsChanged = store.appState.beatmaps_downloaded !== newState.beatmaps_downloaded;

    if (statusChanged) store.appState.status = newState.status;
    if (store.appState.osu_running !== newState.osu_running) store.appState.osu_running = newState.osu_running;
    if (store.appState.requests_proxied !== newState.requests_proxied) store.appState.requests_proxied = newState.requests_proxied;
    if (downloadsChanged) store.appState.beatmaps_downloaded = newState.beatmaps_downloaded;
    if (store.appState.last_error !== newState.last_error) store.appState.last_error = newState.last_error;

    if (statusChanged || downloadsChanged) {
      updateTrayStatus(newState.status, newState.beatmaps_downloaded);
    }
  } catch (e) {
    console.error("Failed to refresh status:", e);
    setError("refresh status", e);
  }
}

export async function checkOsuRunning(): Promise<boolean> {
  try {
    const running = await invoke<boolean>("is_osu_running_cmd");
    if (store.appState.osu_running !== running) {
      store.appState.osu_running = running;
    }
    return running;
  } catch (e) {
    console.error("Failed to check osu! status:", e);
    setError("check osu! status", e);
    return false;
  }
}

export async function getLogs(): Promise<LogEntry[]> {
  startLoading("getLogs");
  try {
    const logs = await invoke<LogEntry[]>("get_logs", { count: null });
    store.logs = logs;
    return logs;
  } catch (e) {
    console.error("Failed to get logs:", e);
    setError("get logs", e);
    return [];
  } finally {
    stopLoading("getLogs");
  }
}

export async function getLogsSince(lastId: number): Promise<LogEntry[]> {
  try {
    const newLogs = await invoke<LogEntry[]>("get_logs_since", { lastId });
    if (newLogs.length > 0) {
      store.logs = [...store.logs, ...newLogs];
    }
    return newLogs;
  } catch {
    return getLogs();
  }
}

export function getLastLogId(): number {
  const logs = store.logs;
  if (logs.length === 0) return 0;
  return logs[logs.length - 1].id ?? 0;
}

export async function clearLogs(): Promise<void> {
  startLoading("clearLogs");
  try {
    await invoke("clear_logs");
    store.logs = [];
  } catch (e) {
    console.error("Failed to clear logs:", e);
    setError("clear logs", e);
  } finally {
    stopLoading("clearLogs");
  }
}

export async function createDesktopShortcut(): Promise<string | null> {
  startLoading("createShortcut");
  try {
    return await invoke<string>("create_launch_shortcut");
  } catch (e) {
    console.error("Failed to create desktop shortcut:", e);
    setError("create desktop shortcut", e);
    return null;
  } finally {
    stopLoading("createShortcut");
  }
}

export async function checkShortcutExists(): Promise<boolean> {
  try {
    return await invoke<boolean>("check_shortcut_exists");
  } catch (e) {
    console.error("Failed to check shortcut:", e);
    return false;
  }
}

export async function removeDesktopShortcut(): Promise<void> {
  startLoading("removeShortcut");
  try {
    await invoke("remove_launch_shortcut");
  } catch (e) {
    console.error("Failed to remove desktop shortcut:", e);
    setError("remove desktop shortcut", e);
    throw e;
  } finally {
    stopLoading("removeShortcut");
  }
}
