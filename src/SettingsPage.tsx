import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Settings, Dictionary, HistoryItem, ActiveShortcut, AuthKind } from "./types";
import {
  getSettings, saveSettings,
  getDictionary, saveDictionary,
  listHistory, clearHistory,
  saveApiKey, hasApiKey,
  checkAccessibility, openAccessibilitySettings,
  activeShortcut,
} from "./invoke";
import "./settings.css";

type Pane = "general" | "models" | "dictionary" | "history" | "apikey";

const PANES: { id: Pane; icon: string; label: string }[] = [
  { id: "general",    icon: "◈",  label: "General"    },
  { id: "models",     icon: "⚙",  label: "Models"     },
  { id: "dictionary", icon: "≡",  label: "Dictionary" },
  { id: "history",    icon: "◷",  label: "History"    },
  { id: "apikey",     icon: "⊙",  label: "API Key"    },
];

export function SettingsPage() {
  const [pane, setPane] = useState<Pane>("general");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [dict, setDict] = useState<Dictionary>({ entries: [] });
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [commonKey, setCommonKey] = useState("");
  const [sttKey, setSttKey] = useState("");
  const [llmKey, setLlmKey] = useState("");
  const [keyExists, setKeyExists] = useState(false);
  const [sttKeyExists, setSttKeyExists] = useState(false);
  const [llmKeyExists, setLlmKeyExists] = useState(false);
  const [accessibilityOk, setAccessibilityOk] = useState(true);
  const [listenerState, setListenerState] = useState<ActiveShortcut | null>(null);
  const [banner, setBanner] = useState<{ type: "saved" | "error"; text: string } | null>(null);

  const refreshListenerState = () =>
    activeShortcut().then(setListenerState).catch(console.error);

  const refreshKeyStatus = () => {
    hasApiKey("common").then(setKeyExists).catch(console.error);
    hasApiKey("stt").then(setSttKeyExists).catch(console.error);
    hasApiKey("llm").then(setLlmKeyExists).catch(console.error);
  };

  useEffect(() => {
    getSettings()
      .then((s) => setSettings({ ...s, shortcut: s.shortcut.toLowerCase() }))
      .catch(console.error);
    getDictionary().then(setDict).catch(console.error);
    refreshKeyStatus();
    checkAccessibility().then(setAccessibilityOk).catch(console.error);
    refreshListenerState();

    const ul1 = listen<string>("shortcut-error", (e) => {
      flash("error", `ショートカット初期化失敗: ${e.payload}`);
      refreshListenerState();
    });
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

  const handleSaveApiKey = async (provider: "stt" | "llm" | "common", key: string) => {
    try {
      await saveApiKey(key, provider);
      refreshKeyStatus();
      if (provider === "stt") setSttKey("");
      else if (provider === "llm") setLlmKey("");
      else setCommonKey("");
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
          {pane === "models" && (
            <ModelsPane
              settings={settings}
              onChange={setSettings}
              onSave={handleSaveSettings}
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
              separateKeys={settings.separate_api_keys}
              commonKey={commonKey}
              sttKey={sttKey}
              llmKey={llmKey}
              keyExists={keyExists}
              sttKeyExists={sttKeyExists}
              llmKeyExists={llmKeyExists}
              onCommonKeyChange={setCommonKey}
              onSttKeyChange={setSttKey}
              onLlmKeyChange={setLlmKey}
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
            <button className="btn-secondary" onClick={() => openAccessibilitySettings()}>
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
              {listenerState.status === "ok" && `✓ 有効: ${listenerState.shortcut}`}
              {listenerState.status === "parse_error" && `❌ ${listenerState.error}`}
              {listenerState.status === "tap_failed" && `❌ ${listenerState.error}`}
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

      <div className="action-bar">
        <button className="btn-primary" onClick={onSave}>保存</button>
      </div>
    </div>
  );
}

// --------------- Models ---------------

const AUTH_KIND_OPTIONS = [
  { value: "bearer",         label: "Bearer トークン" },
  { value: "api_key_header", label: "カスタムヘッダー" },
  { value: "none",           label: "認証なし" },
];

function authKindTag(k: AuthKind): string {
  return k.kind;
}

function buildAuthKind(tag: string, headerName: string): AuthKind {
  if (tag === "api_key_header") return { kind: "api_key_header", header_name: headerName };
  if (tag === "none") return { kind: "none" };
  return { kind: "bearer" };
}

function ProviderSection({
  label,
  config,
  onChange,
}: {
  label: string;
  config: Settings["stt"];
  onChange: (c: Settings["stt"]) => void;
}) {
  const set = (patch: Partial<typeof config>) => onChange({ ...config, ...patch });
  const tag = authKindTag(config.auth_kind);
  const headerName = config.auth_kind.kind === "api_key_header"
    ? config.auth_kind.header_name
    : "";

  return (
    <>
      <div className="section-header">{label}</div>

      <div className="field-row">
        <span className="field-label">Base URL</span>
        <div className="field-control">
          <input
            type="text"
            value={config.base_url}
            onChange={(e) => set({ base_url: e.target.value })}
            placeholder="https://api.openai.com"
          />
        </div>
      </div>

      <div className="field-row">
        <span className="field-label">モデル名</span>
        <div className="field-control">
          <input
            type="text"
            value={config.model}
            onChange={(e) => set({ model: e.target.value })}
          />
        </div>
      </div>

      <div className="field-row">
        <span className="field-label">認証方式</span>
        <div className="field-control">
          <select
            value={tag}
            onChange={(e) =>
              set({ auth_kind: buildAuthKind(e.target.value, headerName) })
            }
          >
            {AUTH_KIND_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
      </div>

      {tag === "api_key_header" && (
        <div className="field-row">
          <span className="field-label">ヘッダー名</span>
          <div className="field-control">
            <input
              type="text"
              value={headerName}
              onChange={(e) =>
                set({ auth_kind: { kind: "api_key_header", header_name: e.target.value } })
              }
              placeholder="x-api-key"
            />
          </div>
        </div>
      )}
    </>
  );
}

function ModelsPane({
  settings, onChange, onSave,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
  onSave: () => void;
}) {
  return (
    <div className="form-section">
      <div className="pane-title">モデル設定</div>
      <p className="models-desc">
        STT と LLM 補正で使うエンドポイント・モデル名・認証方式を設定します。<br />
        OpenAI 公式 / 社内エンドポイント / ローカル LLM など OpenAI 互換 API に対応しています。
      </p>

      <ProviderSection
        label="STT (音声認識)"
        config={settings.stt}
        onChange={(stt) => onChange({ ...settings, stt })}
      />

      <div className="divider" />

      <ProviderSection
        label="LLM (辞書補正)"
        config={settings.llm}
        onChange={(llm) => onChange({ ...settings, llm })}
      />

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
  separateKeys,
  commonKey, sttKey, llmKey,
  keyExists, sttKeyExists, llmKeyExists,
  onCommonKeyChange, onSttKeyChange, onLlmKeyChange,
  onSave,
}: {
  separateKeys: boolean;
  commonKey: string; sttKey: string; llmKey: string;
  keyExists: boolean; sttKeyExists: boolean; llmKeyExists: boolean;
  onCommonKeyChange: (k: string) => void;
  onSttKeyChange: (k: string) => void;
  onLlmKeyChange: (k: string) => void;
  onSave: (provider: "stt" | "llm" | "common", key: string) => void;
}) {
  return (
    <div className="form-section">
      <div className="pane-title">API キー</div>

      {!separateKeys ? (
        <>
          {keyExists && (
            <div className="apikey-status">
              <span className="apikey-status-icon">✅</span>
              <span>APIキーは macOS Keychain に保存済みです</span>
            </div>
          )}
          <div className="field-row">
            <span className="field-label">共通 API キー</span>
            <div className="field-control">
              <input
                type="password"
                value={commonKey}
                onChange={(e) => onCommonKeyChange(e.target.value)}
                placeholder={keyExists ? "新しいキーで上書きする場合に入力…" : "sk-…"}
              />
            </div>
          </div>
          <div className="action-bar">
            <button
              className="btn-primary"
              onClick={() => onSave("common", commonKey)}
              disabled={!commonKey.trim()}
            >
              キーを保存
            </button>
          </div>
        </>
      ) : (
        <>
          <div className="section-header">STT (音声認識) キー</div>
          {sttKeyExists && (
            <div className="apikey-status">
              <span>✅ STT キーが Keychain に保存済み</span>
            </div>
          )}
          <div className="field-row">
            <span className="field-label">STT API キー</span>
            <div className="field-control">
              <input
                type="password"
                value={sttKey}
                onChange={(e) => onSttKeyChange(e.target.value)}
                placeholder={sttKeyExists ? "上書きする場合に入力…" : "sk-…"}
              />
            </div>
          </div>
          <div className="action-bar">
            <button
              className="btn-primary"
              onClick={() => onSave("stt", sttKey)}
              disabled={!sttKey.trim()}
            >
              STT キーを保存
            </button>
          </div>

          <div className="divider" />
          <div className="section-header">LLM (辞書補正) キー</div>
          {llmKeyExists && (
            <div className="apikey-status">
              <span>✅ LLM キーが Keychain に保存済み</span>
            </div>
          )}
          <div className="field-row">
            <span className="field-label">LLM API キー</span>
            <div className="field-control">
              <input
                type="password"
                value={llmKey}
                onChange={(e) => onLlmKeyChange(e.target.value)}
                placeholder={llmKeyExists ? "上書きする場合に入力…" : "sk-…"}
              />
            </div>
          </div>
          <div className="action-bar">
            <button
              className="btn-primary"
              onClick={() => onSave("llm", llmKey)}
              disabled={!llmKey.trim()}
            >
              LLM キーを保存
            </button>
          </div>
        </>
      )}

      <div className="hint">
        キーは macOS Keychain (<code>jp.co.cyberagent.coatype</code>) に保存されます。<br />
        環境変数 <code>COATYPE_API_KEY</code> が設定されている場合はそちらが優先されます。<br />
        STT/LLM を個別に設定する場合は <strong>Models</strong> タブの「STT/LLM」設定で
        <code>separate_api_keys</code> を有効化してください（設定ファイルで直接指定可能）。
      </div>
    </div>
  );
}
