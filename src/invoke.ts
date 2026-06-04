import { invoke } from "@tauri-apps/api/core";
import type { Settings, Dictionary, HistoryItem } from "./types";

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });
export const getDictionary = () => invoke<Dictionary>("get_dictionary");
export const saveDictionary = (dict: Dictionary) =>
  invoke<void>("save_dictionary", { dict });
export const listHistory = (limit: number) =>
  invoke<HistoryItem[]>("list_history", { limit });
export const clearHistory = () => invoke<void>("clear_history");
export const saveApiKey = (key: string) =>
  invoke<void>("save_api_key", { key });
export const hasApiKey = () => invoke<boolean>("has_api_key");
