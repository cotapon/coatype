import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Settings, Dictionary, HistoryItem, ActiveShortcut } from "./types";
import {
  getSettings, saveSettings,
  getDictionary, saveDictionary,
  listHistory, clearHistory,
  saveApiKey, hasApiKey,
  checkAccessibility, openAccessibilitySettings,
  activeShortcut,
} from "./invoke";
import "./settings.css";

type Pane = "general" | "dictionary" | "history" | "apikey";

const PANES: { id: Pane; icon: string; label: string }[] = [
  { id: "general",    icon: "◈",  label: "General"    },
  { id: "dictionary", icon: "≡",  label: "Dictionary" },
  { id: "history",    icon: "◷",  label: "History"    },
  { id: "apikey",     icon: "⊙",  label: "API Key"    },
];

export function SettingsPage() {
  const [pane, setPane] = useState<Pane>("general");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [dict, setDict] = useState<Dictionary>({ entries: [] });
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [keyExists, setKeyExists] = useState(false);
  const [accessibilityOk, setAccessibilityOk] = useState(true);
  const [listenerState, setListenerState] = useState<ActiveShortcut | null>(null);
  const [banner, setBanner] = useState<{ type: "saved" | "error"; text: string } | null>(null);

  const refreshListenerState = () =>
    activeShortcut().then(setListenerState).catch(console.error);

  useEffect(() => {
    getSettings()
      .then((s) => setSettings({ ...s, shortcut: s.shortcut.toLowerCase() }))
      .catch(console.error);
    getDictionary().then(setDict).catch(console.error);
    hasApiKey().then(setKeyExists).catch(console.error);
    checkAccessibility().then(setAccessibilityOk).catch(console.error);
    refreshListenerState();

    // shortcut-error: リスナー初期化失敗 (parse エラー or CGEventTap 失敗)
    const ul1 = listen<string>("shortcut-error", (e) => {
      flash("error", `ショートカット初期化失敗: ${e.payload}`);
      refreshListenerState();
    });
    // error: pipeline 側のエラー (マイク権限失敗など)
    const ul2 = listen<string>("error", (e) => {
      flash("error", e.payload);
    });

    return () => {
      ul1.then((fn) => fn());
      ul2.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (pane === "history") listHistory(100).then(setHistory).catch(console.error);
  }, [pane]);

  const flash = (type: "saved" | "error", text: string) => {
    setBanner({ type, text });
    if (type === "saved") setTimeout(() => setBanner(null), 2000);
  };

  const handleSaveSettings = async () => {
    if (!settings) return;
    try {
      await saveSettings(settings);
      flash("saved", "保存しました ✓");
      refreshListenerState();
    }
    catch (e) { flash("error", String(e)); }
  };

  const handleSaveDict = async () => {
    try { await saveDictionary(dict); flash("saved", "保存しました ✓"); }
    catch (e) { flash("error", String(e)); }
  };

  const handleSaveApiKey = async () => {
    try {
      await saveApiKey(apiKey);
      setKeyExists(true);
      setApiKey("");
      flash("saved", "APIキーを保存しました ✓");
    } catch (e) { flash("error", String(e)); }
  };

  const handleClearHistory = async () => {
    try { await clearHistory(); setHistory([]); }
    catch (e) { flash("error", String(e)); }
  };

  if (!settings) return <div className="loading">読み込み中…</div>;

  return (
    <div className="app">
      {banner && (
        <div className={`banner ${banner.type}`}>
          <span>{banner.text}</span>
          <button onClick={() => setBanner(null)}>✕</button>
        </div>
      )}
      <div className="main-layout">
        <nav className="sidebar">
          {PANES.map((p) => (
            <button
              key={p.id}
              className={`sidebar-btn${pane === p.id ? " active" : ""}`}
              onClick={() => setPane(p.id)}
            >
              <span className="sidebar-icon">{p.icon}</span>
              {p.label}
            </button>
          ))}
        </nav>

        <div className="content">
          {pane === "general" && (
            <GeneralPane
              settings={settings}
              onChange={setSettings}
              onSave={handleSaveSettings}
              accessibilityOk={accessibilityOk}
              onRefreshAccessibility={() =>
                checkAccessibility().then(setAccessibilityOk).catch(console.error)
              }
              listenerState={listenerState}
            />
          )}
          {pane === "dictionary" && (
            <DictionaryPane dict={dict} onChange={setDict} onSave={handleSaveDict} />
          )}
          {pane === "history" && (
            <HistoryPane items={history} onClear={handleClearHistory} />
          )}
          {pane === "apikey" && (
            <ApiKeyPane
              apiKey={apiKey}
              keyExists={keyExists}
              onChange={setApiKey}
              onSave={handleSaveApiKey}
            />
          )}
        </div>
      </div>
    </div>
  );
}

// --------------- General ---------------

const SHORTCUT_OPTIONS = [
  { value: "rightoption",  label: "Right Option ⌥" },
  { value: "leftoption",   label: "Left Option ⌥L" },
  { value: "rightcontrol", label: "Right Control ⌃R" },
  { value: "leftcontrol",  label: "Left Control ⌃L" },
  { value: "f5", label: "F5" },
  { value: "f6", label: "F6" },
  { value: "f7", label: "F7" },
  { value: "f8", label: "F8" },
];

function GeneralPane({
  settings, onChange, onSave, accessibilityOk, onRefreshAccessibility, listenerState,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
  onSave: () => void;
  accessibilityOk: boolean;
  onRefreshAccessibility: () => void;
  listenerState: ActiveShortcut | null;
}) {
  const set = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });

  return (
    <div className="form-section">
      <div className="pane-title">一般設定</div>

      {!accessibilityOk && (
        <div className="accessibility-warning">
          <div className="accessibility-warning-title">⚠️ アクセシビリティ権限が必要です</div>
          <div className="accessibility-warning-body">
            グローバルショートカットを使うには「システム設定 → プライバシーとセキュリティ → アクセシビリティ」で
            CoAType を許可してください。
          </div>
          <div className="accessibility-warning-actions">
            <button
              className="btn-secondary"
              onClick={() => openAccessibilitySettings()}
            >
              システム設定を開く
            </button>
            <button className="btn-secondary" onClick={onRefreshAccessibility}>
              再確認
            </button>
          </div>
        </div>
      )}

      <div className="section-header">文字起こし</div>

      <div className="field-row">
        <span className="field-label">言語コード</span>
        <div className="field-control">
          <input
            type="text"
            value={settings.language}
            onChange={(e) => set({ language: e.target.value })}
            placeholder="ja"
          />
        </div>
      </div>

      <div className="divider" />
      <div className="section-header">ショートカット</div>

      <div className="field-row">
        <span className="field-label">トリガーキー</span>
        <div className="field-control">
          <select
            value={settings.shortcut}
            onChange={(e) => set({ shortcut: e.target.value })}
          >
            {SHORTCUT_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
          {listenerState && (
            <div className={`listener-badge listener-badge-${listenerState.status}`}>
              {listenerState.status === "starting" && "⏳ 起動中..."}
              {listenerState.status === "ok" &&
                `✓ 有効: ${listenerState.shortcut}`}
              {listenerState.status === "parse_error" &&
                `❌ ${listenerState.error}`}
              {listenerState.status === "tap_failed" &&
                `❌ ${listenerState.error}`}
            </div>
          )}
        </div>
      </div>

      <div className="field-row">
        <span className="field-label">入力モード</span>
        <div className="field-control">
          <select
            value={settings.trigger_mode}
            onChange={(e) =>
              set({ trigger_mode: e.target.value as Settings["trigger_mode"] })
            }
          >
            <option value="push_to_talk">長押し (Push-to-Talk)</option>
            <option value="toggle">トグル</option>
          </select>
        </div>
      </div>

      <div className="divider" />
      <div className="section-header">オプション</div>

      <div>
        <div className="checkbox-row">
          <input
            type="checkbox"
            id="translate"
            checked={settings.translate_mode}
            onChange={(e) => set({ translate_mode: e.target.checked })}
          />
          <label className="checkbox-label" htmlFor="translate">英語に翻訳する</label>
        </div>
        <div className="checkbox-desc">/v1/audio/translations エンドポイントを使用</div>
      </div>

      <div>
        <div className="checkbox-row">
          <input
            type="checkbox"
            id="llm"
            checked={settings.llm_correct}
            onChange={(e) => set({ llm_correct: e.target.checked })}
          />
          <label className="checkbox-label" htmlFor="llm">LLM辞書補正 (実験的)</label>
        </div>
        <div className="checkbox-desc">文字起こし後にLLMで辞書と照合して補正します</div>
      </div>

      <div className="divider" />
      <div className="section-header">API</div>

      <div className="field-row">
        <span className="field-label">API Base URL</span>
        <div className="field-control">
          <input
            type="text"
            value={settings.api_base ?? ""}
            onChange={(e) => set({ api_base: e.target.value })}
          />
        </div>
      </div>

      <div className="action-bar">
        <button className="btn-primary" onClick={onSave}>保存</button>
      </div>
    </div>
  );
}

// --------------- Dictionary ---------------

function DictionaryPane({
  dict, onChange, onSave,
}: { dict: Dictionary; onChange: (d: Dictionary) => void; onSave: () => void }) {
  const add = () => onChange({ entries: [...dict.entries, { from: "", to: "" }] });
  const remove = (i: number) =>
    onChange({ entries: dict.entries.filter((_, idx) => idx !== i) });
  const update = (i: number, field: "from" | "to", val: string) =>
    onChange({
      entries: dict.entries.map((e, idx) =>
        idx === i ? { ...e, [field]: val } : e
      ),
    });

  return (
    <div className="form-section">
      <div className="pane-title">カスタム辞書</div>
      <p style={{ fontSize: 12, color: "#636366", lineHeight: 1.5 }}>
        文字起こし結果に対して完全一致で置換します。LLM補正をONにすると文脈も考慮します。
      </p>

      <table className="dict-table">
        <thead>
          <tr>
            <th>変換前 (from)</th>
            <th>変換後 (to)</th>
            <th style={{ width: 36 }}></th>
          </tr>
        </thead>
        <tbody>
          {dict.entries.length === 0 ? (
            <tr>
              <td colSpan={3} className="dict-empty">
                エントリがありません。「+ 追加」で登録してください。
              </td>
            </tr>
          ) : (
            dict.entries.map((entry, i) => (
              <tr key={i}>
                <td>
                  <input
                    value={entry.from}
                    onChange={(e) => update(i, "from", e.target.value)}
                    placeholder="例: ずんだもん"
                  />
                </td>
                <td>
                  <input
                    value={entry.to}
                    onChange={(e) => update(i, "to", e.target.value)}
                    placeholder="例: ずんだモン"
                  />
                </td>
                <td>
                  <button className="btn-icon" onClick={() => remove(i)}>✕</button>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      <div className="action-bar">
        <button className="btn-secondary" onClick={add}>+ 追加</button>
        <button className="btn-primary" onClick={onSave}>保存</button>
      </div>
    </div>
  );
}

// --------------- History ---------------

function HistoryPane({ items, onClear }: { items: HistoryItem[]; onClear: () => void }) {
  return (
    <div className="form-section">
      <div className="pane-title">文字起こし履歴</div>

      <div className="action-bar" style={{ borderTop: "none", marginTop: 0, paddingTop: 0 }}>
        <button className="btn-danger" onClick={onClear} disabled={items.length === 0}>
          全件削除
        </button>
      </div>

      {items.length === 0 ? (
        <div className="history-empty">履歴がありません</div>
      ) : (
        <ul className="history-list">
          {items.map((item) => (
            <li key={item.id} className="history-item">
              <div className="history-text">{item.text}</div>
              <div className="history-meta">
                {item.created_at} · {item.language}
                {item.translated ? " → 英語" : ""} · {item.duration_ms}ms
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// --------------- API Key ---------------

function ApiKeyPane({
  apiKey, keyExists, onChange, onSave,
}: {
  apiKey: string;
  keyExists: boolean;
  onChange: (k: string) => void;
  onSave: () => void;
}) {
  return (
    <div className="form-section">
      <div className="pane-title">API キー</div>

      {keyExists && (
        <div className="apikey-status">
          <span className="apikey-status-icon">✅</span>
          <span>APIキーは macOS Keychain に保存済みです</span>
        </div>
      )}

      <div className="field-row">
        <span className="field-label">APIキー</span>
        <div className="field-control">
          <input
            type="password"
            value={apiKey}
            onChange={(e) => onChange(e.target.value)}
            placeholder={keyExists ? "新しいキーで上書きする場合に入力…" : "mlp-…"}
          />
        </div>
      </div>

      <div className="hint">
        キーは macOS Keychain (<code>jp.co.cyberagent.coatype</code>) に保存されます。<br />
        環境変数 <code>COATYPE_API_KEY</code> が設定されている場合はそちらが優先されます。
      </div>

      <div className="action-bar">
        <button
          className="btn-primary"
          onClick={onSave}
          disabled={!apiKey.trim()}
        >
          キーを保存
        </button>
      </div>
    </div>
  );
}
