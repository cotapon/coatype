export interface Settings {
  language: string;
  shortcut: string;
  trigger_mode: "push_to_talk" | "toggle";
  translate_mode: boolean;
  llm_correct: boolean;
  api_base: string;
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
