export type AuthKind =
  | { kind: "bearer" }
  | { kind: "api_key_header"; header_name: string }
  | { kind: "none" };

export interface ProviderConfig {
  base_url: string;
  model: string;
  auth_kind: AuthKind;
}

export type ActionKind = "start_record" | "hands_free" | "cancel" | "paste_last";

export interface KeyBinding {
  id: string;
  action: ActionKind;
  combo: string;
  enabled: boolean;
}

export interface Settings {
  language: string;
  bindings: KeyBinding[];
  translate_mode: boolean;
  llm_correct: boolean;
  stt: ProviderConfig;
  llm: ProviderConfig;
  separate_api_keys: boolean;
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
  status: "starting" | "ok" | "tap_failed";
  error: string | null;
}

export const ACTION_LABELS: Record<ActionKind, string> = {
  start_record: "録音",
  hands_free: "ハンズフリー",
  cancel: "キャンセル",
  paste_last: "最後の文字起こしを貼り付け",
};

/** KeyboardEvent.code → rdev 互換文字列への変換テーブル */
export const CODE_TO_COMBO: Record<string, string> = {
  AltRight: "rightoption",
  AltLeft: "leftoption",
  ControlRight: "rightcontrol",
  ControlLeft: "leftcontrol",
  ShiftLeft: "leftshift",
  ShiftRight: "rightshift",
  MetaLeft: "leftmeta",
  MetaRight: "rightmeta",
  Space: "space",
  Escape: "escape",
  Enter: "enter",
  Tab: "tab",
  Home: "home",
  End: "end",
  F1: "f1",
  F2: "f2",
  F3: "f3",
  F4: "f4",
  F5: "f5",
  F6: "f6",
  F7: "f7",
  F8: "f8",
  F9: "f9",
  F10: "f10",
  F11: "f11",
  F12: "f12",
  KeyA: "a",
  KeyB: "b",
  KeyC: "c",
  KeyD: "d",
  KeyE: "e",
  KeyF: "f",
  KeyG: "g",
  KeyH: "h",
  KeyI: "i",
  KeyJ: "j",
  KeyK: "k",
  KeyL: "l",
  KeyM: "m",
  KeyN: "n",
  KeyO: "o",
  KeyP: "p",
  KeyQ: "q",
  KeyR: "r",
  KeyS: "s",
  KeyT: "t",
  KeyU: "u",
  KeyV: "v",
  KeyW: "w",
  KeyX: "x",
  KeyY: "y",
  KeyZ: "z",
  Digit0: "0",
  Digit1: "1",
  Digit2: "2",
  Digit3: "3",
  Digit4: "4",
  Digit5: "5",
  Digit6: "6",
  Digit7: "7",
  Digit8: "8",
  Digit9: "9",
};

/** コンボ文字列を表示用ラベルに変換 ("leftmeta+leftcontrol+v" → "Left ⌘ + Left ⌃ + V") */
export function comboToLabel(combo: string): string {
  const partLabel: Record<string, string> = {
    rightoption: "Right ⌥",
    leftoption: "Left ⌥",
    rightcontrol: "Right ⌃",
    leftcontrol: "Left ⌃",
    leftshift: "Left ⇧",
    rightshift: "Right ⇧",
    leftmeta: "Left ⌘",
    rightmeta: "Right ⌘",
    space: "Space",
    escape: "Escape",
    enter: "Enter",
    tab: "Tab",
    home: "Home",
    end: "End",
    fn: "Fn",
  };
  return combo
    .split("+")
    .map((p) => {
      const t = p.trim().toLowerCase();
      if (partLabel[t]) return partLabel[t];
      if (/^f\d+$/.test(t)) return t.toUpperCase();
      return t.toUpperCase();
    })
    .join(" + ");
}

/** IME 衝突の可能性があるコンボかどうかを判定 */
export function detectImeConflict(combo: string): boolean {
  const lower = combo.toLowerCase();
  const parts = lower.split("+").map((p) => p.trim());
  if (parts.length < 2) return false;
  const hasShift = parts.some((p) => p === "leftshift" || p === "rightshift" || p === "shift");
  if (!hasShift) return false;
  const last = parts[parts.length - 1];
  return (last.length === 1 && /[a-z]/.test(last)) || last === "space";
}
