export type AuthKind =
  | { kind: "bearer" }
  | { kind: "api_key_header"; header_name: string }
  | { kind: "none" };

export interface ProviderConfig {
  base_url: string;
  model: string;
  auth_kind: AuthKind;
}

export type ActionKind = "start_record" | "hands_free" | "cancel";

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
  stt: ProviderConfig;
  show_overlay: boolean;
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

/** 実行中の OS。ラベル表示の記号切り替え専用 (combo の永続化・パースには影響しない)。
 *  起動時に getPlatform() の結果で setPlatform() を1回呼ぶまでは "macos" のまま (既存挙動維持)。 */
let currentPlatform: "macos" | "windows" | "linux" = "macos";

/** アプリ起動時に1回呼ぶ。未知の値は無視する。 */
export function setPlatform(os: string): void {
  if (os === "macos" || os === "windows" || os === "linux") {
    currentPlatform = os;
  }
}

/** 修飾キーの簡潔ラベル (comboToLabel 用)。macOS は記号、Windows/Linux は英語表記。 */
const PART_LABEL_SHORT: Record<"macos" | "windows" | "linux", Record<string, string>> = {
  macos: {
    rightoption: "Right ⌥",
    leftoption: "Left ⌥",
    rightcontrol: "Right ⌃",
    leftcontrol: "Left ⌃",
    leftshift: "Left ⇧",
    rightshift: "Right ⇧",
    leftmeta: "Left ⌘",
    rightmeta: "Right ⌘",
  },
  windows: {
    rightoption: "Right Alt",
    leftoption: "Left Alt",
    rightcontrol: "Right Ctrl",
    leftcontrol: "Left Ctrl",
    leftshift: "Left Shift",
    rightshift: "Right Shift",
    leftmeta: "Left Win",
    rightmeta: "Right Win",
  },
  linux: {
    rightoption: "Right Alt",
    leftoption: "Left Alt",
    rightcontrol: "Right Ctrl",
    leftcontrol: "Left Ctrl",
    leftshift: "Left Shift",
    rightshift: "Right Shift",
    leftmeta: "Left Super",
    rightmeta: "Right Super",
  },
};

/** 修飾キーの冗長ラベル (comboToVerboseLabel 用)。macOS は記号+英単語、Windows/Linux は英語表記のみ。
 *  Winロゴ記号 (⊞) はフォント欠落しうるため使わず、プレーンな "Win" テキストにする。 */
const PART_LABEL_VERBOSE: Record<"macos" | "windows" | "linux", Record<string, string>> = {
  macos: {
    rightoption: "⌥ Right Option",
    leftoption: "⌥ Left Option",
    rightcontrol: "⌃ Right Control",
    leftcontrol: "⌃ Left Control",
    leftshift: "⇧ Left Shift",
    rightshift: "⇧ Right Shift",
    leftmeta: "⌘ Left Command",
    rightmeta: "⌘ Right Command",
  },
  windows: {
    rightoption: "Right Alt",
    leftoption: "Left Alt",
    rightcontrol: "Right Ctrl",
    leftcontrol: "Left Ctrl",
    leftshift: "Left Shift",
    rightshift: "Right Shift",
    leftmeta: "Left Win",
    rightmeta: "Right Win",
  },
  linux: {
    rightoption: "Right Alt",
    leftoption: "Left Alt",
    rightcontrol: "Right Ctrl",
    leftcontrol: "Left Ctrl",
    leftshift: "Left Shift",
    rightshift: "Right Shift",
    leftmeta: "Left Super",
    rightmeta: "Right Super",
  },
};

const COMMON_PART_LABEL: Record<string, string> = {
  space: "Space",
  escape: "Escape",
  enter: "Enter",
  tab: "Tab",
  home: "Home",
  end: "End",
  fn: "Fn",
};

/** コンボ文字列を表示用ラベルに変換 ("leftmeta+leftcontrol+v" → "Left ⌘ + Left ⌃ + V")。
 *  修飾キー表記は現在の OS (setPlatform で設定) に応じて切り替わる。 */
export function comboToLabel(combo: string): string {
  const partLabel = { ...COMMON_PART_LABEL, ...PART_LABEL_SHORT[currentPlatform] };
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

/** コンボ文字列を冗長ラベルに変換 ("rightoption" → "⌥ Right Option")。
 *  ステータスカードやキーバインドのチップ表示用に、記号 + 英単語で示す (macOS)。
 *  Windows/Linux では記号を使わず英語表記のみ。 */
export function comboToVerboseLabel(combo: string): string {
  const partLabel = { ...COMMON_PART_LABEL, ...PART_LABEL_VERBOSE[currentPlatform] };
  return combo
    .split("+")
    .map((p) => {
      const t = p.trim().toLowerCase();
      if (partLabel[t]) return partLabel[t];
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
