import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateState {
  checking: boolean;
  available: boolean;
  downloading: boolean;
  progress: number;
  error: string | null;
  version: string | null;
  releaseNotes: string | null;
  dismissed: boolean;
}

const defaultState: UpdateState = {
  checking: false,
  available: false,
  downloading: false,
  progress: 0,
  error: null,
  version: null,
  releaseNotes: null,
  dismissed: false,
};

export const updateStore = $state<UpdateState>({ ...defaultState });

let currentUpdate: Update | null = null;

export function isUpdateAvailable(): boolean {
  return updateStore.available && !updateStore.dismissed;
}

export function isDownloading(): boolean {
  return updateStore.downloading;
}

export async function checkForUpdates(): Promise<void> {
  if (updateStore.checking || updateStore.downloading) return;

  updateStore.checking = true;
  updateStore.error = null;

  try {
    const update = await check();

    if (update) {
      currentUpdate = update;
      updateStore.available = true;
      updateStore.version = update.version;
      updateStore.releaseNotes = update.body ?? null;
      updateStore.dismissed = false;
      console.log(`Update available: ${update.version}`);
    } else {
      updateStore.available = false;
      updateStore.version = null;
      updateStore.releaseNotes = null;
      console.log("No updates available");
    }
  } catch (e) {
    console.error("Failed to check for updates:", e);
    updateStore.error = String(e);
  } finally {
    updateStore.checking = false;
  }
}

export async function downloadAndInstall(): Promise<void> {
  if (!currentUpdate || updateStore.downloading) return;

  updateStore.downloading = true;
  updateStore.progress = 0;
  updateStore.error = null;

  try {
    let totalSize = 0;
    let downloadedSize = 0;

    await currentUpdate.downloadAndInstall((event) => {
      if (event.event === "Started") {
        totalSize = event.data.contentLength ?? 0;
        console.log(`Download started, size: ${totalSize}`);
      } else if (event.event === "Progress") {
        downloadedSize += event.data.chunkLength;
        if (totalSize > 0) {
          updateStore.progress = Math.round((downloadedSize / totalSize) * 100);
        }
      } else if (event.event === "Finished") {
        updateStore.progress = 100;
        console.log("Download complete");
      }
    });

    console.log("Update installed, relaunching...");
    await relaunch();
  } catch (e) {
    console.error("Failed to download/install update:", e);
    updateStore.error = String(e);
    updateStore.downloading = false;
  }
}

export function dismissUpdate(): void {
  updateStore.dismissed = true;
}

export function resetUpdateState(): void {
  Object.assign(updateStore, defaultState);
  currentUpdate = null;
}
