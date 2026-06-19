import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { Update } from "@tauri-apps/plugin-updater";

export type { Update };

/**
 * アップデートをチェックする。
 * 新しいバージョンがあれば Update オブジェクトを返す。なければ null。
 */
export async function checkForUpdate(): Promise<Update | null> {
  return await check();
}

/**
 * アップデートをダウンロード・インストールしてアプリを再起動する。
 * @param update checkForUpdate() の戻り値
 * @param onProgress ダウンロード進捗コールバック (downloaded, total)
 */
export async function downloadAndInstallUpdate(
  update: Update,
  onProgress?: (downloaded: number, total: number | null) => void,
): Promise<void> {
  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        onProgress?.(0, event.data.contentLength ?? null);
        break;
      case "Progress":
        onProgress?.(event.data.chunkLength, null);
        break;
      case "Finished":
        break;
    }
  });
  await relaunch();
}
