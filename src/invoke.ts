import { invoke } from "@tauri-apps/api/core";
import type { Settings, Dictionary, HistoryItem, ActiveShortcut } from "./types";

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });
export const getDictionary = () => invoke<Dictionary>("get_dictionary");
export const saveDictionary = (dict: Dictionary) =>
  invoke<void>("save_dictionary", { dict });
export const listHistory = (limit: number) =>
  invoke<HistoryItem[]>("list_history", { limit });
export const clearHistory = () => invoke<void>("clear_history");

type ApiKeyProvider = "stt" | "llm" | "common";

export const saveApiKey = (key: string, provider: ApiKeyProvider = "common") =>
  invoke<void>("save_api_key", { provider, key });
export const hasApiKey = (provider: ApiKeyProvider = "common") =>
  invoke<boolean>("has_api_key", { provider });

export const checkAccessibility = () => invoke<boolean>("check_accessibility");
export const openAccessibilitySettings = () =>
  invoke<void>("open_accessibility_settings");
export const activeShortcut = () => invoke<ActiveShortcut>("active_shortcut");
