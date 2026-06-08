export type AuthKind =
  | { kind: "bearer" }
  | { kind: "api_key_header"; header_name: string }
  | { kind: "none" };

export interface ProviderConfig {
  base_url: string;
  model: string;
  auth_kind: AuthKind;
}

export interface Settings {
  language: string;
  shortcut: string;
  trigger_mode: "push_to_talk" | "toggle";
  translate_mode: boolean;
  llm_correct: boolean;
  stt: ProviderConfig;
  llm: ProviderConfig;
  separate_api_keys: boolean;
  /** @deprecated マイグレーション用。新規 JSON には含まれない */
  api_base?: string;
}

export interface DictionaryEntry {
  from: string;
  to: string;
}

export interface Dictionary {
  entries: DictionaryEntry[];
}

export interface HistoryItem {
  id: number;
  text: string;
  language: string;
  translated: boolean;
  duration_ms: number;
  created_at: string;
}

export interface ActiveShortcut {
  shortcut: string;
  trigger_mode: string;
  status: "starting" | "ok" | "parse_error" | "tap_failed";
  error: string | null;
}
