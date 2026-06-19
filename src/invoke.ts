import { invoke } from "@tauri-apps/api/core";
import type { Settings, Dictionary, HistoryItem, ActiveShortcut } from "./types";

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });
export const getDictionary = () => invoke<Dictionary>("get_dictionary");
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
export const hasApiKey = () => invoke<boolean>("has_api_key");

export const checkAccessibility = () => invoke<boolean>("check_accessibility");
export const openAccessibilitySettings = () =>
  invoke<void>("open_accessibility_settings");
export const activeShortcut = () => invoke<ActiveShortcut>("active_shortcut");
export const setListenerPaused = (paused: boolean) =>
  invoke<void>("set_listener_paused", { paused });

export const startTestRecording = () => invoke<void>("start_test_recording");
export const stopTestRecording = () => invoke<string>("stop_test_recording");
