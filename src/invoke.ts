import { invoke } from "@tauri-apps/api/core";
import type { Settings, Dictionary, HistoryItem, ActiveShortcut } from "./types";

/**
 * Tauri IPC 呼び出しをリトライ付きで実行する。
 *
 * Windows では WebView2 (webview) が Rust の setup() 完了より先に起動するため、
 * State が未登録のうちに invoke が届いて reject されることがある。
 * 数百 ms のリトライで setup 完了を待ち、その後の呼び出しは確実に成功する。
 */
async function invokeWithRetry<T>(
  cmd: string,
  args?: Record<string, unknown>,
  retries = 5,
  delayMs = 200,
): Promise<T> {
  let lastError: unknown;
  for (let i = 0; i < retries; i++) {
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      lastError = e;
      if (i < retries - 1) {
        await new Promise((r) => setTimeout(r, delayMs));
      }
    }
  }
  throw lastError;
}

// 起動時に呼ばれる読み取り系コマンドはリトライ付きで呼ぶ。
// 保存系・副作用系はリトライ不要なので通常の invoke のまま。
export const getSettings = () => invokeWithRetry<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });
export const getDictionary = () => invokeWithRetry<Dictionary>("get_dictionary");
export const saveDictionary = (dict: Dictionary) =>
  invoke<void>("save_dictionary", { dict });
export const importDictionary = (path: string) =>
  invoke<Dictionary>("import_dictionary", { path });
export const exportDictionary = (path: string) =>
  invoke<void>("export_dictionary", { path });
export const listHistory = (limit: number) =>
  invoke<HistoryItem[]>("list_history", { limit });
export const clearHistory = () => invoke<void>("clear_history");

export const saveApiKey = (key: string) =>
  invoke<void>("save_api_key", { key });
export const hasApiKey = () => invokeWithRetry<boolean>("has_api_key");

export const checkAccessibility = () =>
  invokeWithRetry<boolean>("check_accessibility");
export const openAccessibilitySettings = () =>
  invoke<void>("open_accessibility_settings");
export const activeShortcut = () =>
  invokeWithRetry<ActiveShortcut>("active_shortcut");
export const setListenerPaused = (paused: boolean) =>
  invoke<void>("set_listener_paused", { paused });

export const startTestRecording = () => invoke<void>("start_test_recording");
export const stopTestRecording = () => invoke<string>("stop_test_recording");

export const openUrl = (url: string) => invoke<void>("open_url", { url });
