import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export interface AppUpdateInfo {
  available: boolean;
  version: string | null;
  currentVersion: string | null;
  date: string | null;
  notes: string;
}

let pendingUpdate: Update | null = null;

export async function checkForAppUpdate(): Promise<AppUpdateInfo> {
  const update = await check({ timeout: 15_000, allowDowngrades: false });
  if (!update) {
    pendingUpdate = null;
    return { available: false, version: null, currentVersion: null, date: null, notes: "" };
  }
  if (update.version.includes("-")) {
    const currentVersion = update.currentVersion;
    await update.close();
    pendingUpdate = null;
    return { available: false, version: null, currentVersion, date: null, notes: "" };
  }
  if (pendingUpdate) await pendingUpdate.close();
  pendingUpdate = update;
  return {
    available: true,
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date ?? null,
    notes: update.body ?? "릴리스 노트가 없습니다."
  };
}

export async function installPendingUpdate(onProgress: (percent: number | null) => void): Promise<void> {
  if (!pendingUpdate) throw new Error("설치할 업데이트가 없습니다. 다시 확인해 주세요.");
  let downloaded = 0;
  let total = 0;
  await pendingUpdate.downloadAndInstall((event) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? 0;
      downloaded = 0;
      onProgress(total > 0 ? 0 : null);
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress(total > 0 ? Math.min(100, Math.round(downloaded / total * 100)) : null);
    } else if (event.event === "Finished") {
      onProgress(100);
    }
  }, { timeout: 120_000 });
  await relaunch();
}
