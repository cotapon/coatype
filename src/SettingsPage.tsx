import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  Alert,
  Button,
  Card,
  CloseButton,
  Description,
  Input,
  ListBox,
  Modal,
  Select,
  Spinner,
  Switch,
  TextField,
} from "@heroui/react";
import type {
  Settings,
  Dictionary,
  HistoryItem,
  ActiveShortcut,
  AuthKind,
  KeyBinding,
  ActionKind,
} from "./types";
import {
  CODE_TO_COMBO,
  comboToLabel,
  comboToVerboseLabel,
  detectImeConflict,
} from "./types";
import {
  getSettings, saveSettings,
  getDictionary, saveDictionary,
  listHistory, clearHistory,
  saveApiKey, hasApiKey,
  checkAccessibility, openAccessibilitySettings,
  activeShortcut,
  setListenerPaused,
  startTestRecording, stopTestRecording,
} from "./invoke";
import { LANGUAGES } from "./languages";

type Pane = "general" | "models" | "apikey" | "dictionary" | "history";

// --------------- アイコン (軽量インライン SVG) ---------------

function Svg({ className = "size-4", children }: { className?: string; children: React.ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

const IconHome = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M4 11.5 12 4l8 7.5" />
    <path d="M6 10v9.5h12V10" />
  </Svg>
);
const IconCube = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" />
    <path d="M4 7.5 12 12l8-4.5" />
    <path d="M12 12v9" />
  </Svg>
);
const IconKey = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="8.5" cy="8.5" r="4" />
    <line x1="11.4" y1="11.4" x2="20" y2="20" />
    <line x1="17.5" y1="17.5" x2="19.5" y2="15.5" />
    <line x1="15" y1="15" x2="17" y2="13" />
  </Svg>
);
const IconBook = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M5 4.5h9a2 2 0 0 1 2 2V20a2 2 0 0 0-2-2H5z" />
    <path d="M19 6.5V18a2 2 0 0 0-2 2" />
  </Svg>
);
const IconClock = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="12" cy="12" r="8.2" />
    <path d="M12 7.5v5l3.2 2" />
  </Svg>
);
const IconMic = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="9" y="3" width="6" height="11" rx="3" />
    <path d="M6 11a6 6 0 0 0 12 0" />
    <line x1="12" y1="17" x2="12" y2="21" />
    <line x1="9" y1="21" x2="15" y2="21" />
  </Svg>
);
const IconGlobe = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="12" cy="12" r="8.5" />
    <path d="M3.5 12h17" />
    <path d="M12 3.5c2.5 2.3 2.5 14.7 0 17M12 3.5c-2.5 2.3-2.5 14.7 0 17" />
  </Svg>
);
const IconKeyboard = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="3" y="6" width="18" height="12" rx="2" />
    <line x1="7" y1="10" x2="7" y2="10" />
    <line x1="11" y1="10" x2="11" y2="10" />
    <line x1="15" y1="10" x2="15" y2="10" />
    <line x1="8" y1="14" x2="16" y2="14" />
  </Svg>
);
const IconHand = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M9 11V5.5a1.5 1.5 0 0 1 3 0V11" />
    <path d="M12 10.5v-1a1.5 1.5 0 0 1 3 0V11" />
    <path d="M15 10.5a1.5 1.5 0 0 1 3 0V15a5 5 0 0 1-5 5h-1.5a4 4 0 0 1-3-1.4L5 15.5a1.6 1.6 0 0 1 2.3-2.2L9 15V8a1.5 1.5 0 0 1 3 0" />
  </Svg>
);
const IconX = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="6" y1="6" x2="18" y2="18" />
    <line x1="18" y1="6" x2="6" y2="18" />
  </Svg>
);
const IconClipboard = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="6" y="4.5" width="12" height="16" rx="2" />
    <path d="M9 4.5V3.5h6v1" />
    <line x1="9" y1="9.5" x2="15" y2="9.5" />
    <line x1="9" y1="13" x2="15" y2="13" />
    <line x1="9" y1="16.5" x2="13" y2="16.5" />
  </Svg>
);
const IconWaveform = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="5" y1="10" x2="5" y2="14" />
    <line x1="9" y1="7" x2="9" y2="17" />
    <line x1="12" y1="4" x2="12" y2="20" />
    <line x1="15" y1="7" x2="15" y2="17" />
    <line x1="19" y1="10" x2="19" y2="14" />
  </Svg>
);
const IconPencil = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M4 20h4L19 9l-4-4L4 16v4z" />
    <path d="M14 6l4 4" />
  </Svg>
);
const IconTrash = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M5 7h14" />
    <path d="M9.5 7V5h5v2" />
    <path d="M7 7l.9 12h8.2L17 7" />
    <line x1="10.5" y1="10.5" x2="10.5" y2="15.5" />
    <line x1="13.5" y1="10.5" x2="13.5" y2="15.5" />
  </Svg>
);
const IconPlus = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  </Svg>
);
const IconWarning = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M12 3.5l8.5 15h-17z" />
    <line x1="12" y1="9.5" x2="12" y2="13.5" />
    <circle cx="12" cy="16.5" r="0.8" fill="currentColor" stroke="none" />
  </Svg>
);
const IconCheck = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="12" cy="12" r="8.5" />
    <path d="M8.5 12.2l2.3 2.3 4.7-4.8" />
  </Svg>
);

// --------------- 共通レイアウト要素 ---------------

/** ペイン見出し (タイトル + サブタイトル) */
function PaneHeader({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div className="mb-6">
      <h1 className="text-[26px] font-bold tracking-tight text-foreground">{title}</h1>
      <p className="mt-1 text-sm text-muted">{subtitle}</p>
    </div>
  );
}

/** 太字の中見出しを持つセクション */
function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-7">
      <h2 className="mb-3 text-[15px] font-bold text-foreground">{title}</h2>
      {children}
    </section>
  );
}

/** 白いカードコンテナ (rounded-xl + 枠線 + 影) */
function Panel({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return (
    <div
      className={`rounded-xl border border-border bg-surface shadow-[var(--surface-shadow)] ${className}`}
    >
      {children}
    </div>
  );
}

/** 小さな丸アイコンタイル (淡いアクセント背景) */
function IconTile({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return (
    <div
      className={`flex size-9 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent ${className}`}
    >
      {children}
    </div>
  );
}

const PANES: { id: Pane; Icon: React.ComponentType<{ className?: string }>; label: string }[] = [
  { id: "general",    Icon: IconHome,  label: "一般設定"   },
  { id: "models",     Icon: IconCube,  label: "モデル設定" },
  { id: "apikey",     Icon: IconKey,   label: "APIキー"    },
  { id: "dictionary", Icon: IconBook,  label: "辞書・語彙" },
  { id: "history",    Icon: IconClock, label: "履歴"       },
];

const PANE_META: Record<Pane, { title: string; subtitle: string }> = {
  general:    { title: "一般設定",       subtitle: "CoAType の基本設定をカスタマイズします" },
  models:     { title: "モデル設定",     subtitle: "STT と LLM 補正で使うエンドポイント・モデルを設定します" },
  apikey:     { title: "API キー",       subtitle: "API キーは macOS Keychain に安全に保存されます" },
  dictionary: { title: "辞書・語彙",     subtitle: "文字起こし結果を辞書で自動補正します" },
  history:    { title: "文字起こし履歴", subtitle: "過去の文字起こし結果を確認します" },
};

export function SettingsPage() {
  const [pane, setPane] = useState<Pane>("general");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [initialSettings, setInitialSettings] = useState<Settings | null>(null);
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

  const settingsLoaded = useRef(false);
  const dictLoaded = useRef(false);

  const refreshListenerState = () =>
    activeShortcut().then(setListenerState).catch(console.error);

  const refreshKeyStatus = () => {
    hasApiKey("common").then(setKeyExists).catch(console.error);
    hasApiKey("stt").then(setSttKeyExists).catch(console.error);
    hasApiKey("llm").then(setLlmKeyExists).catch(console.error);
  };

  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettings(s);
        setInitialSettings(s);
      })
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

  // 設定の自動保存 (変更を 500ms debounce して保存)
  useEffect(() => {
    if (!settings) return;
    if (!settingsLoaded.current) {
      settingsLoaded.current = true;
      return;
    }
    const t = setTimeout(() => {
      saveSettings(settings)
        .then(refreshListenerState)
        .catch((e) => flash("error", String(e)));
    }, 500);
    return () => clearTimeout(t);
  }, [settings]);

  // 辞書の自動保存
  useEffect(() => {
    if (!dictLoaded.current) {
      dictLoaded.current = true;
      return;
    }
    const t = setTimeout(() => {
      saveDictionary(dict).catch((e) => flash("error", String(e)));
    }, 500);
    return () => clearTimeout(t);
  }, [dict]);

  const flash = (type: "saved" | "error", text: string) => {
    setBanner({ type, text });
    if (type === "saved") setTimeout(() => setBanner(null), 2000);
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

  const handleReset = () => {
    if (initialSettings) setSettings(initialSettings);
  };

  const handleDone = () => {
    getCurrentWebviewWindow().hide().catch(console.error);
  };

  if (!settings) {
    return (
      <div className="fixed inset-0 flex items-center justify-center bg-background text-sm text-muted">
        読み込み中…
      </div>
    );
  }

  return (
    <div className="fixed inset-0 flex flex-col bg-background text-foreground">
      {banner && (
        <div className="fixed left-1/2 top-3 z-50 w-[min(420px,90vw)] -translate-x-1/2">
          <Alert status={banner.type === "saved" ? "success" : "danger"} className="shadow-lg">
            <Alert.Indicator />
            <Alert.Content>
              <Alert.Title>{banner.text}</Alert.Title>
            </Alert.Content>
            <CloseButton onPress={() => setBanner(null)} />
          </Alert>
        </div>
      )}

      <div className="flex min-h-0 flex-1">
        {/* サイドバー */}
        <nav className="flex w-[212px] shrink-0 flex-col px-3 py-4">
          <div className="flex flex-col gap-1">
            {PANES.map(({ id, Icon, label }) => {
              const active = pane === id;
              return (
                <button
                  key={id}
                  onClick={() => setPane(id)}
                  className={`flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors ${
                    active
                      ? "bg-accent text-accent-foreground shadow-sm"
                      : "text-foreground hover:bg-surface-secondary"
                  }`}
                >
                  <Icon className="size-[18px]" />
                  {label}
                </button>
              );
            })}
          </div>

          <div className="mt-auto flex items-center gap-2.5 px-1 pt-4">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-surface text-accent">
              <IconWaveform className="size-[18px]" />
            </div>
            <div className="min-w-0 leading-tight">
              <div className="text-sm font-semibold text-foreground">CoAType</div>
              <div className="text-xs text-muted">Version 1.0.0</div>
            </div>
          </div>
        </nav>

        {/* メインパネル */}
        <main className="min-w-0 flex-1 py-3 pr-3">
          <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-border bg-surface shadow-[var(--surface-shadow)]">
            <div className="min-h-0 flex-1 overflow-y-auto px-8 py-7">
              <div className="mx-auto w-full max-w-[760px]">
                <PaneHeader {...PANE_META[pane]} />

                {pane === "general" && (
                  <GeneralPane
                    settings={settings}
                    onChange={setSettings}
                    accessibilityOk={accessibilityOk}
                    onRefreshAccessibility={() =>
                      checkAccessibility().then(setAccessibilityOk).catch(console.error)
                    }
                    listenerState={listenerState}
                    onError={(t) => flash("error", t)}
                  />
                )}
                {pane === "models" && (
                  <ModelsPane settings={settings} onChange={setSettings} />
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
                {pane === "dictionary" && (
                  <DictionaryPane dict={dict} onChange={setDict} />
                )}
                {pane === "history" && (
                  <HistoryPane items={history} onClear={handleClearHistory} />
                )}
              </div>
            </div>
          </div>
        </main>
      </div>

      {/* フッターバー */}
      <footer className="flex shrink-0 items-center justify-between border-t border-border px-6 py-3">
        <div className="flex items-center gap-2 text-sm text-muted">
          <IconCheck className="size-4 text-accent" />
          すべての変更は自動的に保存されます
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onPress={handleReset}>
            リセット
          </Button>
          <Button variant="primary" onPress={handleDone}>
            完了
          </Button>
        </div>
      </footer>
    </div>
  );
}

// --------------- General ---------------

function GeneralPane({
  settings, onChange, accessibilityOk, onRefreshAccessibility, listenerState, onError,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
  accessibilityOk: boolean;
  onRefreshAccessibility: () => void;
  listenerState: ActiveShortcut | null;
  onError: (text: string) => void;
}) {
  const set = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });

  const langOption = LANGUAGES.find((l) => l.code === settings.language);
  const langLabel = langOption ? `${langOption.label} (${langOption.code})` : settings.language;

  const startBinding = settings.bindings.find((b) => b.action === "start_record" && b.enabled)
    ?? settings.bindings.find((b) => b.action === "start_record");
  const recKeyLabel = startBinding ? comboToVerboseLabel(startBinding.combo) : "未設定";

  return (
    <>
      {!accessibilityOk && (
        <Alert status="warning" className="mb-6 w-full">
          <Alert.Indicator />
          <Alert.Content>
            <Alert.Title>アクセシビリティ権限が必要です</Alert.Title>
            <Alert.Description>
              グローバルショートカットを使うには「システム設定 → プライバシーとセキュリティ → アクセシビリティ」で
              CoAType を許可してください。
            </Alert.Description>
            <div className="mt-2 flex gap-2">
              <Button size="sm" variant="secondary" onPress={() => openAccessibilitySettings()}>
                システム設定を開く
              </Button>
              <Button size="sm" variant="outline" onPress={onRefreshAccessibility}>
                再確認
              </Button>
            </div>
          </Alert.Content>
        </Alert>
      )}

      <StatusCard
        listenerState={listenerState}
        langLabel={langLabel}
        recKeyLabel={recKeyLabel}
        modelLabel={settings.stt.model || "未設定"}
        onError={onError}
      />

      <Section title="言語">
        <Panel className="p-4">
          <div className="flex items-center gap-3">
            <IconTile><IconGlobe className="size-[18px]" /></IconTile>
            <span className="text-sm font-medium text-foreground">言語</span>
            <Select
              aria-label="言語"
              variant="secondary"
              className="ml-2 flex-1"
              value={settings.language}
              onChange={(v) => set({ language: String(v) })}
            >
              <Select.Trigger>
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover>
                <ListBox>
                  {!LANGUAGES.some((l) => l.code === settings.language) && (
                    <ListBox.Item id={settings.language} textValue={settings.language}>
                      {settings.language}
                      <ListBox.ItemIndicator />
                    </ListBox.Item>
                  )}
                  {LANGUAGES.map((l) => (
                    <ListBox.Item key={l.code} id={l.code} textValue={`${l.label} (${l.code})`}>
                      {l.label} ({l.code})
                      <ListBox.ItemIndicator />
                    </ListBox.Item>
                  ))}
                </ListBox>
              </Select.Popover>
            </Select>
          </div>
          <p className="mt-2.5 pl-12 text-xs text-muted">文字起こしに使用する言語を選択します</p>
        </Panel>
      </Section>

      <Section title="キーバインド">
        <KeybindingsSection
          bindings={settings.bindings}
          onChange={(bindings) => set({ bindings })}
        />
      </Section>

      <Section title="オプション">
        <Panel className="divide-y divide-separator">
          <OptionRow
            title="英語に翻訳する"
            description="/v1/audio/translations エンドポイントを使用"
            isSelected={settings.translate_mode}
            onChange={(v) => set({ translate_mode: v })}
          />
          <OptionRow
            title="LLM辞書補正（実験的）"
            description="文字起こし後にLLMで辞書と照合して補正します"
            isSelected={settings.llm_correct}
            onChange={(v) => set({ llm_correct: v })}
          />
          <OptionRow
            title="録音中オーバーレイを表示する"
            description="録音・処理中にインジケーターを画面下部に表示します"
            isSelected={settings.show_overlay}
            onChange={(v) => set({ show_overlay: v })}
          />
        </Panel>
      </Section>
    </>
  );
}

function OptionRow({
  title, description, isSelected, onChange,
}: {
  title: string;
  description: string;
  isSelected: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="px-4 py-3.5">
      <Switch isSelected={isSelected} onChange={onChange}>
        <Switch.Content className="items-start gap-3">
          <Switch.Control className="mt-0.5">
            <Switch.Thumb />
          </Switch.Control>
          <div className="flex flex-col">
            <span className="text-sm font-medium text-foreground">{title}</span>
            <Description className="text-xs text-muted">{description}</Description>
          </div>
        </Switch.Content>
      </Switch>
    </div>
  );
}

// --------------- StatusCard (録音テスト付き) ---------------

type TestState = "idle" | "recording" | "transcribing";

function StatusCard({
  listenerState, langLabel, recKeyLabel, modelLabel, onError,
}: {
  listenerState: ActiveShortcut | null;
  langLabel: string;
  recKeyLabel: string;
  modelLabel: string;
  onError: (text: string) => void;
}) {
  const [testState, setTestState] = useState<TestState>("idle");
  const [testResult, setTestResult] = useState<string | null>(null);

  const handleTest = async () => {
    if (testState === "idle") {
      setTestResult(null);
      try {
        await setListenerPaused(true);
        await startTestRecording();
        setTestState("recording");
      } catch (e) {
        await setListenerPaused(false).catch(() => {});
        onError(`録音テスト開始に失敗: ${e}`);
      }
    } else if (testState === "recording") {
      setTestState("transcribing");
      try {
        const text = await stopTestRecording();
        setTestResult(text);
      } catch (e) {
        onError(`録音テストに失敗: ${e}`);
      } finally {
        await setListenerPaused(false).catch(() => {});
        setTestState("idle");
      }
    }
  };

  // 録音テスト中はリスナー状態に関わらずテスト用の表示を優先する
  const recording = testState === "recording";
  const transcribing = testState === "transcribing";

  let dotClass = "bg-success";
  let title = "Listening Ready";
  let subtitle = "音声入力の準備ができています";

  if (recording) {
    dotClass = "bg-danger";
    title = "録音中…";
    subtitle = "もう一度「停止」を押すと文字起こしします";
  } else if (transcribing) {
    dotClass = "bg-warning";
    title = "文字起こし中…";
    subtitle = "STT で音声を処理しています";
  } else if (listenerState?.status === "tap_failed") {
    dotClass = "bg-danger";
    title = "リスナー停止";
    subtitle = listenerState.error ?? "ショートカットを初期化できませんでした";
  } else if (listenerState?.status === "starting") {
    dotClass = "bg-muted";
    title = "起動中…";
    subtitle = "ショートカットリスナーを起動しています";
  }

  return (
    <div className="mb-7 rounded-2xl border border-accent/15 bg-gradient-to-br from-accent-soft to-surface p-5">
      <div className="flex items-start gap-4">
        <div className="relative shrink-0">
          <div className="flex size-14 items-center justify-center rounded-full bg-accent text-accent-foreground shadow-sm">
            <IconMic className="size-7" />
          </div>
          <span className={`absolute bottom-0.5 right-0.5 size-3.5 rounded-full border-2 border-surface ${dotClass}`} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-lg font-bold text-foreground">{title}</div>
          <div className="truncate text-sm text-muted">{subtitle}</div>
        </div>
        <Button
          variant="secondary"
          className="shrink-0 gap-1.5"
          onPress={handleTest}
          isDisabled={transcribing}
        >
          {transcribing ? (
            <Spinner size="sm" color="current" />
          ) : (
            <span className="text-accent">
              <IconWaveform className="size-4" />
            </span>
          )}
          {recording ? "停止" : "録音テスト"}
        </Button>
      </div>

      <div className="mt-5 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <SummaryItem Icon={IconGlobe} label="言語" value={langLabel} />
        <SummaryItem Icon={IconKeyboard} label="録音キー" value={recKeyLabel} />
        <SummaryItem Icon={IconCube} label="モデル" value={modelLabel} />
      </div>

      {testResult !== null && (
        <div className="mt-4 rounded-xl border border-border bg-surface px-4 py-3">
          <div className="mb-1 text-xs font-medium text-muted">認識結果</div>
          <div className="text-sm break-words text-foreground">
            {testResult || "（音声が検出されませんでした）"}
          </div>
        </div>
      )}
    </div>
  );
}

function SummaryItem({
  Icon, label, value,
}: {
  Icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center gap-2.5">
      <IconTile><Icon className="size-[18px]" /></IconTile>
      <div className="min-w-0">
        <div className="text-xs text-muted">{label}</div>
        <div className="truncate text-sm font-medium text-foreground">{value}</div>
      </div>
    </div>
  );
}

// --------------- KeybindingsSection ---------------

const ACTION_ORDER: ActionKind[] = ["start_record", "hands_free", "cancel", "paste_last"];

const ACTION_ICONS: Record<ActionKind, React.ComponentType<{ className?: string }>> = {
  start_record: IconMic,
  hands_free: IconHand,
  cancel: IconX,
  paste_last: IconClipboard,
};

const ACTION_ROW_LABELS: Record<ActionKind, string> = {
  start_record: "録音開始",
  hands_free: "ハンズフリー開始",
  cancel: "キャンセル",
  paste_last: "最後の文字起こしを貼り付け",
};

type CaptureState =
  | { mode: "set"; action: ActionKind }
  | { mode: "edit"; id: string; action: ActionKind; combo: string }
  | { mode: "add" };

function KeybindingsSection({
  bindings, onChange,
}: {
  bindings: KeyBinding[];
  onChange: (b: KeyBinding[]) => void;
}) {
  const [capture, setCapture] = useState<CaptureState | null>(null);

  const byAction = (action: ActionKind) => bindings.filter((b) => b.action === action);

  const addBinding = (action: ActionKind, combo: string) => {
    onChange([...bindings, { id: crypto.randomUUID(), action, combo, enabled: true }]);
  };
  const updateBinding = (id: string, combo: string) => {
    onChange(bindings.map((b) => (b.id === id ? { ...b, combo } : b)));
  };
  const toggleBinding = (id: string) => {
    onChange(bindings.map((b) => (b.id === id ? { ...b, enabled: !b.enabled } : b)));
  };
  const removeBinding = (id: string) => {
    onChange(bindings.filter((b) => b.id !== id));
  };

  const handleConfirm = (action: ActionKind, combo: string) => {
    if (!capture) return;
    if (capture.mode === "edit") updateBinding(capture.id, combo);
    else addBinding(action, combo);
    setCapture(null);
  };

  return (
    <>
      <Panel className="overflow-hidden">
        <div className="divide-y divide-separator">
          {ACTION_ORDER.map((action) => {
            const rows = byAction(action);
            const Icon = ACTION_ICONS[action];
            const isPrimary = action === "start_record";

            if (rows.length === 0) {
              return (
                <div key={action} className="flex items-center gap-3 px-4 py-3">
                  <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-surface-secondary text-muted">
                    <Icon className="size-[18px]" />
                  </div>
                  <span className="text-sm font-medium text-foreground">{ACTION_ROW_LABELS[action]}</span>
                  <div className="ml-auto flex items-center gap-2">
                    <span className="text-sm text-muted">未設定</span>
                    <Button size="sm" variant="secondary" onPress={() => setCapture({ mode: "set", action })}>
                      設定
                    </Button>
                    <Button isIconOnly size="sm" variant="secondary" isDisabled aria-label="削除">
                      <IconTrash className="size-4" />
                    </Button>
                  </div>
                </div>
              );
            }

            return rows.map((binding) => (
              <div key={binding.id} className="flex items-center gap-3 px-4 py-3">
                <div
                  className={`flex size-9 shrink-0 items-center justify-center rounded-full ${
                    isPrimary ? "bg-accent text-accent-foreground" : "bg-surface-secondary text-muted"
                  }`}
                >
                  <Icon className="size-[18px]" />
                </div>
                <span className="text-sm font-medium text-foreground">{ACTION_ROW_LABELS[action]}</span>
                <div className="ml-auto flex items-center gap-2.5">
                  <span
                    className={`inline-flex items-center rounded-lg bg-surface-secondary px-2.5 py-1 text-sm font-medium text-foreground ${
                      binding.enabled ? "" : "opacity-50"
                    }`}
                  >
                    {comboToVerboseLabel(binding.combo)}
                  </span>
                  {detectImeConflict(binding.combo) && (
                    <span className="text-warning" title="IMEと衝突する可能性があります">
                      <IconWarning className="size-4" />
                    </span>
                  )}
                  <button
                    onClick={() => toggleBinding(binding.id)}
                    title="クリックで有効 / 無効を切り替え"
                    className={`rounded-md px-2 py-0.5 text-xs font-medium transition-opacity hover:opacity-80 ${
                      binding.enabled
                        ? "bg-success-soft text-success"
                        : "bg-surface-secondary text-muted"
                    }`}
                  >
                    {binding.enabled ? "有効" : "無効"}
                  </button>
                  <div className="ml-1 flex items-center gap-1.5">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="secondary"
                      onPress={() => setCapture({ mode: "edit", id: binding.id, action, combo: binding.combo })}
                      aria-label="編集"
                    >
                      <IconPencil className="size-4" />
                    </Button>
                    <Button
                      isIconOnly
                      size="sm"
                      variant="secondary"
                      className="text-muted hover:text-danger"
                      onPress={() => removeBinding(binding.id)}
                      aria-label="削除"
                    >
                      <IconTrash className="size-4" />
                    </Button>
                  </div>
                </div>
              </div>
            ));
          })}
        </div>

        <button
          onClick={() => setCapture({ mode: "add" })}
          className="flex w-full items-center justify-center gap-1.5 border-t border-dashed border-border px-4 py-3 text-sm font-medium text-accent transition-colors hover:bg-accent-soft"
        >
          <IconPlus className="size-4" />
          キーバインドを追加
        </button>
      </Panel>

      {capture && (
        <CaptureModal
          presetAction={capture.mode === "add" ? undefined : capture.action}
          initialCombo={capture.mode === "edit" ? capture.combo : undefined}
          onConfirm={handleConfirm}
          onClose={() => setCapture(null)}
        />
      )}
    </>
  );
}

// --------------- CaptureModal ---------------

function CaptureModal({
  presetAction, initialCombo, onConfirm, onClose,
}: {
  presetAction?: ActionKind;
  initialCombo?: string;
  onConfirm: (action: ActionKind, combo: string) => void;
  onClose: () => void;
}) {
  const [selectedAction, setSelectedAction] = useState<ActionKind>(presetAction ?? "start_record");
  const [capturedCombo, setCapturedCombo] = useState<string>(initialCombo ?? "");
  const [displayKeys, setDisplayKeys] = useState<string[]>(
    initialCombo ? initialCombo.split("+") : [],
  );
  const [isCapturing, setIsCapturing] = useState(!initialCombo);
  const heldCodesRef = useRef<string[]>([]);

  // モーダルが開いている間はグローバルリスナーを停止して録音を防ぐ
  useEffect(() => {
    setListenerPaused(true).catch(console.error);
    return () => { setListenerPaused(false).catch(console.error); };
  }, []);

  useEffect(() => {
    if (!isCapturing) return;
    heldCodesRef.current = [];

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      const key = CODE_TO_COMBO[e.code];
      if (!key) return;
      if (!heldCodesRef.current.includes(key)) {
        heldCodesRef.current = [...heldCodesRef.current, key];
        setDisplayKeys([...heldCodesRef.current]);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      const released = CODE_TO_COMBO[e.code];
      if (!released || !heldCodesRef.current.includes(released)) return;
      const modifiers = heldCodesRef.current.filter((k) => k !== released);
      const combo = [...modifiers, released].join("+");
      setCapturedCombo(combo);
      setDisplayKeys([...modifiers, released]);
      setIsCapturing(false);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("keyup", handleKeyUp, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("keyup", handleKeyUp, true);
    };
  }, [isCapturing]);

  const displayText = isCapturing
    ? (displayKeys.length > 0 ? comboToLabel(displayKeys.join("+")) : "キーを押してください...")
    : (capturedCombo ? comboToLabel(capturedCombo) : "");

  const startCapture = () => {
    setIsCapturing(true);
    setDisplayKeys([]);
    heldCodesRef.current = [];
  };

  return (
    <Modal.Backdrop
      variant="blur"
      isOpen
      onOpenChange={(open) => { if (!open) onClose(); }}
    >
      <Modal.Container>
        <Modal.Dialog className="sm:max-w-[400px]">
          <Modal.Header>
            <Modal.Heading>キーバインドを設定</Modal.Heading>
            <p className="mt-1.5 text-sm text-muted">
              使用したいキーの組み合わせを押してください。
            </p>
          </Modal.Header>
          <Modal.Body>
            {!presetAction && (
              <div className="mb-3">
                <label className="mb-1.5 block text-sm font-medium text-foreground">アクション</label>
                <Select
                  aria-label="アクション"
                  variant="secondary"
                  className="w-full"
                  value={selectedAction}
                  onChange={(v) => setSelectedAction(v as ActionKind)}
                >
                  <Select.Trigger>
                    <Select.Value />
                    <Select.Indicator />
                  </Select.Trigger>
                  <Select.Popover>
                    <ListBox>
                      {ACTION_ORDER.map((a) => (
                        <ListBox.Item key={a} id={a} textValue={ACTION_ROW_LABELS[a]}>
                          {ACTION_ROW_LABELS[a]}
                          <ListBox.ItemIndicator />
                        </ListBox.Item>
                      ))}
                    </ListBox>
                  </Select.Popover>
                </Select>
              </div>
            )}

            <button
              onClick={startCapture}
              className={`flex min-h-[52px] w-full items-center justify-center rounded-xl border-2 px-4 py-2 text-center text-[15px] font-medium tracking-wide transition-colors ${
                isCapturing
                  ? "border-accent bg-accent-soft text-muted"
                  : capturedCombo
                    ? "border-success bg-surface-secondary text-foreground"
                    : "border-border bg-surface-secondary text-foreground"
              }`}
            >
              {displayText}
            </button>

            {!isCapturing && capturedCombo && detectImeConflict(capturedCombo) && (
              <Alert status="warning" className="mt-3">
                <Alert.Indicator />
                <Alert.Content>
                  <Alert.Description>
                    このキー組み合わせは日本語IMEと衝突する可能性があります
                  </Alert.Description>
                </Alert.Content>
              </Alert>
            )}
          </Modal.Body>
          <Modal.Footer>
            <Button variant="secondary" onPress={onClose}>
              キャンセル
            </Button>
            <Button
              onPress={() => capturedCombo && onConfirm(selectedAction, capturedCombo)}
              isDisabled={!capturedCombo || isCapturing}
            >
              保存
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
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

function FieldRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 px-4 py-2.5">
      <span className="w-32 shrink-0 text-sm text-foreground">{label}</span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
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
    <Section title={label}>
      <Panel className="divide-y divide-separator">
        <FieldRow label="Base URL">
          <TextField
            aria-label={`${label} Base URL`}
            className="w-full"
            value={config.base_url}
            onChange={(v) => set({ base_url: v })}
          >
            <Input placeholder="https://api.openai.com" />
          </TextField>
        </FieldRow>

        <FieldRow label="モデル名">
          <TextField
            aria-label={`${label} モデル名`}
            className="w-full"
            value={config.model}
            onChange={(v) => set({ model: v })}
          >
            <Input />
          </TextField>
        </FieldRow>

        <FieldRow label="認証方式">
          <Select
            aria-label={`${label} 認証方式`}
            variant="secondary"
            className="w-full"
            value={tag}
            onChange={(v) => set({ auth_kind: buildAuthKind(String(v), headerName) })}
          >
            <Select.Trigger>
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox>
                {AUTH_KIND_OPTIONS.map((o) => (
                  <ListBox.Item key={o.value} id={o.value} textValue={o.label}>
                    {o.label}
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                ))}
              </ListBox>
            </Select.Popover>
          </Select>
        </FieldRow>

        {tag === "api_key_header" && (
          <FieldRow label="ヘッダー名">
            <TextField
              aria-label={`${label} ヘッダー名`}
              className="w-full"
              value={headerName}
              onChange={(v) => set({ auth_kind: { kind: "api_key_header", header_name: v } })}
            >
              <Input placeholder="x-api-key" />
            </TextField>
          </FieldRow>
        )}
      </Panel>
    </Section>
  );
}

function ModelsPane({
  settings, onChange,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
}) {
  return (
    <>
      <p className="mb-5 text-xs leading-relaxed text-muted">
        OpenAI 公式 / 社内エンドポイント / ローカル LLM など OpenAI 互換 API に対応しています。
      </p>

      <ProviderSection
        label="STT (音声認識)"
        config={settings.stt}
        onChange={(stt) => onChange({ ...settings, stt })}
      />

      <ProviderSection
        label="LLM (辞書補正)"
        config={settings.llm}
        onChange={(llm) => onChange({ ...settings, llm })}
      />
    </>
  );
}

// --------------- Dictionary ---------------

function DictionaryPane({
  dict, onChange,
}: { dict: Dictionary; onChange: (d: Dictionary) => void }) {
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
    <>
      <p className="mb-5 text-xs leading-relaxed text-muted">
        文字起こし結果に対して完全一致で置換します。LLM補正をONにすると文脈も考慮します。
      </p>

      <Panel className="overflow-hidden">
        <div className="flex items-center gap-3 bg-surface-secondary px-4 py-2 text-xs font-semibold tracking-wide text-muted uppercase">
          <span className="flex-1">変換前 (from)</span>
          <span className="flex-1">変換後 (to)</span>
          <span className="w-8" />
        </div>
        {dict.entries.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted">
            エントリがありません。「+ 追加」で登録してください。
          </div>
        ) : (
          <div className="divide-y divide-separator">
            {dict.entries.map((entry, i) => (
              <div key={i} className="flex items-center gap-3 px-4 py-2">
                <TextField
                  aria-label={`変換前 ${i + 1}`}
                  className="flex-1"
                  value={entry.from}
                  onChange={(v) => update(i, "from", v)}
                >
                  <Input placeholder="例: ずんだもん" />
                </TextField>
                <TextField
                  aria-label={`変換後 ${i + 1}`}
                  className="flex-1"
                  value={entry.to}
                  onChange={(v) => update(i, "to", v)}
                >
                  <Input placeholder="例: ずんだモン" />
                </TextField>
                <Button
                  isIconOnly
                  size="sm"
                  variant="secondary"
                  className="text-muted hover:text-danger"
                  onPress={() => remove(i)}
                  aria-label="削除"
                >
                  <IconTrash className="size-4" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </Panel>

      <div className="mt-4 flex justify-start">
        <Button variant="secondary" onPress={add}>
          <IconPlus className="size-4" />
          追加
        </Button>
      </div>
    </>
  );
}

// --------------- History ---------------

function HistoryPane({ items, onClear }: { items: HistoryItem[]; onClear: () => void }) {
  const [copiedId, setCopiedId] = useState<number | null>(null);

  const handleCopy = (id: number, text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    });
  };

  return (
    <>
      <div className="mb-4 flex w-full justify-end">
        <Button variant="danger" size="sm" onPress={onClear} isDisabled={items.length === 0}>
          全件削除
        </Button>
      </div>

      {items.length === 0 ? (
        <div className="w-full py-10 text-center text-sm text-muted">履歴がありません</div>
      ) : (
        <ul className="flex w-full flex-col gap-1.5">
          {items.map((item) => (
            <li key={item.id}>
              <Card variant="default" className="w-full">
                <div className="flex items-end gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="leading-snug break-words text-foreground">{item.text}</div>
                    <div className="mt-0.5 text-xs text-muted">
                      {item.created_at} · {item.language}
                      {item.translated ? " → 英語" : ""} · {item.duration_ms}ms
                    </div>
                  </div>
                  <Button
                    size="sm"
                    variant={copiedId === item.id ? "ghost" : "outline"}
                    className={copiedId === item.id ? "text-success" : ""}
                    onPress={() => handleCopy(item.id, item.text)}
                  >
                    {copiedId === item.id ? "✓ コピー済み" : "コピー"}
                  </Button>
                </div>
              </Card>
            </li>
          ))}
        </ul>
      )}
    </>
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
  const savedAlert = (text: string) => (
    <Alert status="success" className="w-full">
      <Alert.Indicator />
      <Alert.Content>
        <Alert.Title>{text}</Alert.Title>
      </Alert.Content>
    </Alert>
  );

  return (
    <>
      {!separateKeys ? (
        <Section title="共通 API キー">
          <div className="flex flex-col gap-3">
            {keyExists && savedAlert("APIキーは macOS Keychain に保存済みです")}
            <Panel className="p-4">
              <TextField
                aria-label="共通 API キー"
                type="password"
                className="w-full"
                value={commonKey}
                onChange={onCommonKeyChange}
              >
                <Input placeholder={keyExists ? "新しいキーで上書きする場合に入力…" : "sk-…"} />
              </TextField>
            </Panel>
            <div className="flex justify-end">
              <Button onPress={() => onSave("common", commonKey)} isDisabled={!commonKey.trim()}>
                キーを保存
              </Button>
            </div>
          </div>
        </Section>
      ) : (
        <>
          <Section title="STT (音声認識) キー">
            <div className="flex flex-col gap-3">
              {sttKeyExists && savedAlert("STT キーが Keychain に保存済み")}
              <Panel className="p-4">
                <TextField
                  aria-label="STT API キー"
                  type="password"
                  className="w-full"
                  value={sttKey}
                  onChange={onSttKeyChange}
                >
                  <Input placeholder={sttKeyExists ? "上書きする場合に入力…" : "sk-…"} />
                </TextField>
              </Panel>
              <div className="flex justify-end">
                <Button onPress={() => onSave("stt", sttKey)} isDisabled={!sttKey.trim()}>
                  STT キーを保存
                </Button>
              </div>
            </div>
          </Section>

          <Section title="LLM (辞書補正) キー">
            <div className="flex flex-col gap-3">
              {llmKeyExists && savedAlert("LLM キーが Keychain に保存済み")}
              <Panel className="p-4">
                <TextField
                  aria-label="LLM API キー"
                  type="password"
                  className="w-full"
                  value={llmKey}
                  onChange={onLlmKeyChange}
                >
                  <Input placeholder={llmKeyExists ? "上書きする場合に入力…" : "sk-…"} />
                </TextField>
              </Panel>
              <div className="flex justify-end">
                <Button onPress={() => onSave("llm", llmKey)} isDisabled={!llmKey.trim()}>
                  LLM キーを保存
                </Button>
              </div>
            </div>
          </Section>
        </>
      )}

      <Card variant="default" className="w-full">
        <p className="text-xs leading-relaxed text-muted">
          キーは macOS Keychain (<code className="rounded bg-surface-secondary px-1 py-0.5 font-mono text-[11px] text-foreground">jp.co.cyberagent.coatype</code>) に保存されます。<br />
          環境変数 <code className="rounded bg-surface-secondary px-1 py-0.5 font-mono text-[11px] text-foreground">COATYPE_API_KEY</code> が設定されている場合はそちらが優先されます。<br />
          STT/LLM を個別に設定する場合は <strong>モデル設定</strong> の
          <code className="rounded bg-surface-secondary px-1 py-0.5 font-mono text-[11px] text-foreground">separate_api_keys</code> を有効化してください（設定ファイルで直接指定可能）。
        </p>
      </Card>
    </>
  );
}
