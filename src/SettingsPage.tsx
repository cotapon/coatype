import { useState, useEffect, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
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
import { CODE_TO_COMBO, comboToLabel, comboToVerboseLabel, detectImeConflict } from "./types";
import {
  getSettings,
  saveSettings,
  getDictionary,
  saveDictionary,
  importDictionary,
  exportDictionary,
  listHistory,
  clearHistory,
  saveApiKey,
  hasApiKey,
  checkAccessibility,
  openAccessibilitySettings,
  activeShortcut,
  setListenerPaused,
  startTestRecording,
  stopTestRecording,
  listInputDevices,
  startMicTest,
  stopMicTest,
  openUrl,
} from "./invoke";
import { open as openFileDialog, save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import { checkForUpdate, downloadAndInstallUpdate } from "./updater";
import type { Update } from "./updater";
import { LANGUAGES } from "./languages";
import logoUrl from "./assets/logo.png";
import {
  IconHome,
  IconCube,
  IconKey,
  IconBook,
  IconClock,
  IconMic,
  IconGlobe,
  IconKeyboard,
  IconHand,
  IconX,
  IconWaveform,
  IconPencil,
  IconTrash,
  IconPlus,
  IconWarning,
  IconCopy,
  IconUpload,
  IconRefresh,
  IconDownload,
} from "./icons";
import { PaneHeader, Section, Panel, IconTile } from "./ui";
import { useTextSelection, type SelectionInfo } from "./hooks/useTextSelection";

type Pane = "general" | "models" | "apikey" | "dictionary" | "history";

const PANES: { id: Pane; Icon: React.ComponentType<{ className?: string }>; label: string }[] = [
  { id: "general", Icon: IconHome, label: "一般設定" },
  { id: "models", Icon: IconCube, label: "モデル設定" },
  { id: "apikey", Icon: IconKey, label: "APIキー" },
  { id: "dictionary", Icon: IconBook, label: "辞書・語彙" },
  { id: "history", Icon: IconClock, label: "履歴" },
];

const PANE_META: Record<Pane, { title: string; subtitle: string }> = {
  general: { title: "一般設定", subtitle: "CoAType の基本設定をカスタマイズします" },
  models: { title: "モデル設定", subtitle: "STT で使うエンドポイント・モデルを設定します" },
  apikey: { title: "APIキー", subtitle: "APIキーは macOS Keychain に安全に保存されます" },
  dictionary: { title: "辞書・語彙", subtitle: "文字起こし結果を辞書で自動補正します" },
  history: { title: "文字起こし履歴", subtitle: "過去の文字起こし結果を確認します" },
};

export function SettingsPage() {
  const [pane, setPane] = useState<Pane>("general");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [initialSettings, setInitialSettings] = useState<Settings | null>(null);
  const [dict, setDict] = useState<Dictionary>({ entries: [] });
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [keyExists, setKeyExists] = useState(false);
  const [accessibilityOk, setAccessibilityOk] = useState(true);
  const [listenerState, setListenerState] = useState<ActiveShortcut | null>(null);
  const [banner, setBanner] = useState<{ type: "saved" | "error"; text: string } | null>(null);
  const [appVersion, setAppVersion] = useState("");
  const [loadError, setLoadError] = useState<string | null>(null);
  // 自動保存 (debounce) の状態表示。トーストほど主張しない常設インジケータで
  // 「保存されたかわからない」を解消する
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved">("idle");
  const saveIndicatorTimer = useRef<number | undefined>(undefined);

  const markSaved = () => {
    setSaveState("saved");
    window.clearTimeout(saveIndicatorTimer.current);
    saveIndicatorTimer.current = window.setTimeout(() => setSaveState("idle"), 1500);
  };

  const settingsLoaded = useRef(false);
  const dictLoaded = useRef(false);

  const refreshListenerState = () => activeShortcut().then(setListenerState).catch(console.error);

  const refreshKeyStatus = () => {
    hasApiKey().then(setKeyExists).catch(console.error);
  };

  const flash = (type: "saved" | "error", text: string) => {
    setBanner({ type, text });
    if (type === "saved") setTimeout(() => setBanner(null), 2000);
  };

  // 設定の初期ロード。invoke.ts の invokeWithRetry により起動レースに対して耐性があるが、
  // 最終的に失敗した場合はエラーUIを出してユーザーが再試行できるようにする。
  const loadSettings = () => {
    setLoadError(null);
    getSettings()
      .then((s) => {
        setSettings(s);
        setInitialSettings(s);
      })
      .catch((e) => setLoadError(String(e)));
  };

  useEffect(() => {
    loadSettings();
    getDictionary()
      .then(setDict)
      .catch((e) => flash("error", `辞書の読み込みに失敗しました: ${e}`));
    refreshKeyStatus();
    checkAccessibility().then(setAccessibilityOk).catch(console.error);
    refreshListenerState();
    getVersion().then(setAppVersion).catch(console.error);

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
    if (pane === "history")
      listHistory(100)
        .then(setHistory)
        .catch((e) => flash("error", `履歴の読み込みに失敗しました: ${e}`));
  }, [pane]);

  // 設定の自動保存 (変更を 500ms debounce して保存)
  useEffect(() => {
    if (!settings) return;
    if (!settingsLoaded.current) {
      settingsLoaded.current = true;
      return;
    }
    const t = setTimeout(() => {
      setSaveState("saving");
      saveSettings(settings)
        .then(() => {
          markSaved();
          return refreshListenerState();
        })
        .catch((e) => {
          setSaveState("idle");
          flash("error", String(e));
        });
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
      setSaveState("saving");
      saveDictionary(dict)
        .then(markSaved)
        .catch((e) => {
          setSaveState("idle");
          flash("error", String(e));
        });
    }, 500);
    return () => clearTimeout(t);
  }, [dict]);

  const handleSaveApiKey = async (key: string) => {
    try {
      await saveApiKey(key);
      refreshKeyStatus();
      setApiKey("");
      flash("saved", "APIキーを保存しました");
    } catch (e) {
      flash("error", String(e));
    }
  };

  const handleClearHistory = async () => {
    try {
      await clearHistory();
      setHistory([]);
    } catch (e) {
      flash("error", String(e));
    }
  };

  const handleRegisterDictEntry = (from: string, to: string) => {
    const isUpdate = dict.entries.some((e) => e.from === from);
    setDict({
      entries: isUpdate
        ? dict.entries.map((e) => (e.from === from ? { ...e, to } : e))
        : [...dict.entries, { from, to }],
    });
    flash("saved", isUpdate ? "辞書を上書きしました" : "辞書に登録しました");
  };

  const handleReset = () => {
    if (initialSettings) setSettings(initialSettings);
  };

  if (!settings) {
    if (loadError) {
      return (
        <div className="fixed inset-0 flex flex-col items-center justify-center gap-4 bg-background text-sm text-foreground">
          <p className="text-danger">設定の読み込みに失敗しました</p>
          <p className="max-w-xs break-all text-center text-xs text-muted">{loadError}</p>
          <button
            onClick={loadSettings}
            className="rounded-md bg-accent px-4 py-2 text-sm text-white hover:opacity-80"
          >
            再試行
          </button>
        </div>
      );
    }
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
          <Alert
            status={banner.type === "saved" ? "success" : "danger"}
            className={`shadow-2xl shadow-black/60 ring-1 ${banner.type === "saved" ? "ring-success/60" : "ring-danger/60"}`}
          >
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
                  aria-current={active ? "page" : undefined}
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

          <div className="mt-auto px-1 pt-4">
            <button
              onClick={() => openUrl("https://github.com/cotapon/coatype/issues/new")}
              className="mb-3 w-full rounded-lg border border-border bg-surface px-3 py-2 text-xs text-muted hover:bg-muted/10 hover:text-foreground transition-colors text-left flex items-center gap-2"
            >
              <svg viewBox="0 0 16 16" className="size-3.5 shrink-0 fill-current" aria-hidden>
                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
              </svg>
              フィードバックを送る
            </button>
            <div className="flex items-center gap-2.5">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-white p-1">
                <img src={logoUrl} alt="CoAType" className="size-full object-contain" />
              </div>
              <div className="min-w-0 leading-tight">
                <div className="text-sm font-semibold text-foreground">CoAType</div>
                <div className="text-xs text-muted">
                  {appVersion ? `Version ${appVersion}` : "—"}
                </div>
              </div>
            </div>
          </div>
        </nav>

        {/* メインパネル */}
        <main className="min-w-0 flex-1 py-3 pr-3">
          <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-border bg-surface shadow-[var(--surface-shadow)]">
            <div className="min-h-0 flex-1 overflow-y-auto px-8 py-7">
              {saveState !== "idle" && (
                <div
                  role="status"
                  className="pointer-events-none fixed right-8 top-6 z-40 rounded-full border border-border bg-surface px-3 py-1 text-xs text-muted shadow-sm"
                >
                  {saveState === "saving" ? "保存中…" : "保存しました ✓"}
                </div>
              )}
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
                    onReset={handleReset}
                  />
                )}
                {pane === "models" && <ModelsPane settings={settings} onChange={setSettings} />}
                {pane === "apikey" && (
                  <ApiKeyPane
                    apiKey={apiKey}
                    keyExists={keyExists}
                    onApiKeyChange={setApiKey}
                    onSave={handleSaveApiKey}
                  />
                )}
                {pane === "dictionary" && (
                  <DictionaryPane dict={dict} onChange={setDict} flash={flash} />
                )}
                {pane === "history" && (
                  <HistoryPane
                    items={history}
                    onClear={handleClearHistory}
                    dict={dict}
                    onRegister={handleRegisterDictEntry}
                  />
                )}
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}

// --------------- General ---------------

function GeneralPane({
  settings,
  onChange,
  accessibilityOk,
  onRefreshAccessibility,
  listenerState,
  onError,
  onReset,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
  accessibilityOk: boolean;
  onRefreshAccessibility: () => void;
  listenerState: ActiveShortcut | null;
  onError: (text: string) => void;
  onReset: () => void;
}) {
  const set = (patch: Partial<Settings>) => onChange({ ...settings, ...patch });
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  // アップデート状態
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "latest" | "error">("idle");

  const handleCheckUpdate = async (silent = false) => {
    setUpdateChecking(true);
    setUpdateStatus("idle");
    setUpdateAvailable(null);
    try {
      const update = await checkForUpdate();
      if (update) {
        setUpdateAvailable(update);
      } else {
        setUpdateStatus("latest");
      }
    } catch (e) {
      if (!silent) onError(`アップデートチェック失敗: ${e}`);
      setUpdateStatus("error");
    } finally {
      setUpdateChecking(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateAvailable) return;
    setUpdateInstalling(true);
    try {
      await downloadAndInstallUpdate(updateAvailable);
      // relaunch が呼ばれるのでここには到達しないが念のため
    } catch (e) {
      onError(`アップデートインストール失敗: ${e}`);
      setUpdateInstalling(false);
    }
  };

  // 設定画面を開いたタイミングで自動チェック（失敗はサイレント）
  useEffect(() => {
    handleCheckUpdate(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const langOption = LANGUAGES.find((l) => l.code === settings.language);
  const langLabel = langOption ? `${langOption.label} (${langOption.code})` : settings.language;

  const startBinding =
    settings.bindings.find((b) => b.action === "start_record" && b.enabled) ??
    settings.bindings.find((b) => b.action === "start_record");
  const recKeyLabel = startBinding ? comboToVerboseLabel(startBinding.combo) : "未設定";

  return (
    <>
      {!accessibilityOk && (
        <Alert status="warning" className="mb-6 w-full">
          <Alert.Indicator />
          <Alert.Content>
            <Alert.Title>アクセシビリティ権限が必要です</Alert.Title>
            <Alert.Description>
              グローバルショートカットを使うには「システム設定 → プライバシーとセキュリティ →
              アクセシビリティ」で CoAType を許可してください。
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

      <MicSection settings={settings} onChange={set} onError={onError} />

      <Section title="言語">
        <Panel className="p-4">
          <div className="flex items-center gap-3">
            <IconTile>
              <IconGlobe className="size-[18px]" />
            </IconTile>
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
            title="録音中オーバーレイを表示する"
            description="録音・処理中にインジケーターを画面下部に表示します"
            isSelected={settings.show_overlay}
            onChange={(v) => set({ show_overlay: v })}
          />
        </Panel>
      </Section>

      <Section title="アップデート">
        <Panel className="p-4">
          {updateAvailable ? (
            <div className="flex flex-col gap-3">
              <div>
                <p className="text-sm font-medium text-foreground">
                  バージョン {updateAvailable.version} が利用可能です
                </p>
                {updateAvailable.body && (
                  <p className="mt-1 whitespace-pre-wrap text-xs text-muted">
                    {updateAvailable.body}
                  </p>
                )}
              </div>
              <div className="flex justify-end">
                <Button
                  variant="primary"
                  onPress={handleInstallUpdate}
                  isDisabled={updateInstalling}
                  className="shrink-0"
                >
                  {updateInstalling ? (
                    <>
                      <Spinner className="size-4" /> インストール中…
                    </>
                  ) : (
                    "今すぐ更新"
                  )}
                </Button>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between gap-4">
              <p className="text-sm text-muted">
                {updateStatus === "latest"
                  ? "最新バージョンを使用しています。"
                  : "新しいバージョンが利用可能か確認します。"}
              </p>
              <Button
                variant="secondary"
                onPress={() => handleCheckUpdate()}
                isDisabled={updateChecking}
                className="shrink-0"
              >
                {updateChecking ? (
                  <>
                    <Spinner className="size-4" /> 確認中…
                  </>
                ) : (
                  "更新を確認"
                )}
              </Button>
            </div>
          )}
        </Panel>
      </Section>

      <Section title="設定のリセット">
        <Panel className="p-4">
          <div className="flex items-center justify-between gap-4">
            <p className="text-sm text-muted">
              この画面を開いた時点の設定に戻します。今回行った変更は破棄されます。
            </p>
            <Button variant="danger" onPress={() => setShowResetConfirm(true)} className="shrink-0">
              設定をリセット
            </Button>
          </div>
        </Panel>
      </Section>

      <p className="mt-2 text-center text-[11px] text-muted">© 2026 cotapon</p>

      {showResetConfirm && (
        <Modal.Backdrop
          variant="blur"
          isOpen
          onOpenChange={(open) => {
            if (!open) setShowResetConfirm(false);
          }}
        >
          <Modal.Container>
            <Modal.Dialog className="sm:max-w-[400px]">
              <Modal.Header>
                <Modal.Heading>設定をリセットしますか？</Modal.Heading>
                <p className="mt-1.5 text-sm text-muted">
                  この画面を開いた時点の設定に戻します。今回行った変更は破棄されます。
                </p>
              </Modal.Header>
              <Modal.Footer>
                <Button variant="secondary" onPress={() => setShowResetConfirm(false)}>
                  キャンセル
                </Button>
                <Button
                  variant="danger"
                  onPress={() => {
                    onReset();
                    setShowResetConfirm(false);
                  }}
                >
                  リセット
                </Button>
              </Modal.Footer>
            </Modal.Dialog>
          </Modal.Container>
        </Modal.Backdrop>
      )}
    </>
  );
}

function OptionRow({
  title,
  description,
  isSelected,
  onChange,
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
  listenerState,
  langLabel,
  recKeyLabel,
  modelLabel,
  onError,
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
          <span
            className={`absolute bottom-0.5 right-0.5 size-3.5 rounded-full border-2 border-surface ${dotClass}`}
          />
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
  Icon,
  label,
  value,
}: {
  Icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center gap-2.5">
      <IconTile>
        <Icon className="size-[18px]" />
      </IconTile>
      <div className="min-w-0">
        <div className="text-xs text-muted">{label}</div>
        <div className="truncate text-sm font-medium text-foreground">{value}</div>
      </div>
    </div>
  );
}

// --------------- KeybindingsSection ---------------

const ACTION_ORDER: ActionKind[] = ["start_record", "hands_free", "cancel"];

const ACTION_ICONS: Record<ActionKind, React.ComponentType<{ className?: string }>> = {
  start_record: IconMic,
  hands_free: IconHand,
  cancel: IconX,
  unknown: IconWarning,
};

const ACTION_ROW_LABELS: Record<ActionKind, string> = {
  start_record: "録音開始",
  hands_free: "ハンズフリー開始",
  cancel: "キャンセル",
  unknown: "不明なアクション",
};

type CaptureState =
  | { mode: "set"; action: ActionKind }
  | { mode: "edit"; id: string; action: ActionKind; combo: string }
  | { mode: "add" };

function KeybindingsSection({
  bindings,
  onChange,
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

            if (rows.length === 0) {
              return (
                <div key={action} className="flex items-center gap-3 px-4 py-3">
                  <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-surface-secondary text-muted">
                    <Icon className="size-[18px]" />
                  </div>
                  <span className="text-sm font-medium text-foreground">
                    {ACTION_ROW_LABELS[action]}
                  </span>
                  <div className="ml-auto flex items-center gap-2">
                    <span className="text-sm text-muted">未設定</span>
                    <Button
                      size="sm"
                      variant="secondary"
                      onPress={() => setCapture({ mode: "set", action })}
                    >
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
                <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-accent text-accent-foreground">
                  <Icon className="size-[18px]" />
                </div>
                <span className="text-sm font-medium text-foreground">
                  {ACTION_ROW_LABELS[action]}
                </span>
                <div className="ml-auto flex items-center gap-2.5">
                  <span className="inline-flex items-center rounded-lg bg-surface-secondary px-2.5 py-1 text-sm font-medium text-foreground">
                    {comboToVerboseLabel(binding.combo)}
                  </span>
                  {detectImeConflict(binding.combo) && (
                    <span className="text-warning" title="IMEと衝突する可能性があります">
                      <IconWarning className="size-4" />
                    </span>
                  )}
                  <div className="ml-1 flex items-center gap-1.5">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="secondary"
                      onPress={() =>
                        setCapture({ mode: "edit", id: binding.id, action, combo: binding.combo })
                      }
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
  presetAction,
  initialCombo,
  onConfirm,
  onClose,
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
    return () => {
      setListenerPaused(false).catch(console.error);
    };
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
    ? displayKeys.length > 0
      ? comboToLabel(displayKeys.join("+"))
      : "キーを押してください..."
    : capturedCombo
      ? comboToLabel(capturedCombo)
      : "";

  const startCapture = () => {
    setIsCapturing(true);
    setDisplayKeys([]);
    heldCodesRef.current = [];
  };

  return (
    <Modal.Backdrop
      variant="blur"
      isOpen
      isKeyboardDismissDisabled={isCapturing}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
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
                <label className="mb-1.5 block text-sm font-medium text-foreground">
                  アクション
                </label>
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

// --------------- マイク選択 / テスト ---------------

/** null (システムのデフォルト) を Select で表現するためのセンチネル値。 */
const MIC_DEFAULT = "__default__";

/** マイクテストのレベルメーターで振幅がフルスケールになる RMS 値。
 *  StatusOverlay の Waveform と同じ LevelAccumulator 由来の値を受け取るため、
 *  同程度の値 (小声でも反応するよう低め) を採用する。 */
const MIC_TEST_LEVEL_CEIL = 0.025;

function MicSection({
  settings,
  onChange,
  onError,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onError: (text: string) => void;
}) {
  const [devices, setDevices] = useState<string[]>([]);
  const [loadingDevices, setLoadingDevices] = useState(false);
  const [showMicTest, setShowMicTest] = useState(false);

  const reloadDevices = () => {
    setLoadingDevices(true);
    listInputDevices()
      .then(setDevices)
      .catch((e) => onError(String(e)))
      .finally(() => setLoadingDevices(false));
  };

  // 初回マウント時に一度だけ取得する。
  useEffect(() => {
    reloadDevices();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selected = settings.input_device ?? MIC_DEFAULT;

  return (
    <Section title="マイク">
      <Panel className="p-4">
        <div className="flex items-center gap-3">
          <IconTile>
            <IconMic className="size-[18px]" />
          </IconTile>
          <span className="text-sm font-medium text-foreground">マイク</span>
          <Select
            aria-label="マイク"
            variant="secondary"
            className="ml-2 flex-1"
            value={selected}
            onChange={(v) => onChange({ input_device: v === MIC_DEFAULT ? null : String(v) })}
          >
            <Select.Trigger>
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox>
                <ListBox.Item id={MIC_DEFAULT} textValue="システムのデフォルト">
                  システムのデフォルト
                  <ListBox.ItemIndicator />
                </ListBox.Item>
                {/* 保存済みのデバイスが一覧から消えている場合でも選択状態を表示し続ける */}
                {settings.input_device && !devices.includes(settings.input_device) && (
                  <ListBox.Item id={settings.input_device} textValue={settings.input_device}>
                    {settings.input_device}
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                )}
                {devices.map((d) => (
                  <ListBox.Item key={d} id={d} textValue={d}>
                    {d}
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                ))}
              </ListBox>
            </Select.Popover>
          </Select>
          <Button
            variant="secondary"
            isDisabled={loadingDevices}
            onPress={reloadDevices}
            aria-label="デバイス一覧を再取得"
          >
            <IconRefresh className="size-4" />
          </Button>
          <Button variant="secondary" onPress={() => setShowMicTest(true)}>
            テスト
          </Button>
        </div>
        <p className="mt-2.5 pl-12 text-xs text-muted">文字起こしに使用するマイクを選択します</p>
      </Panel>

      {showMicTest && (
        <MicTestModal
          device={settings.input_device ?? null}
          onClose={() => setShowMicTest(false)}
        />
      )}
    </Section>
  );
}

function MicTestModal({ device, onClose }: { device: string | null; onClose: () => void }) {
  const [running, setRunning] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const levelRef = useRef(0);

  // マイクテスト中はグローバルショートカットを止め、モーダル終了時に必ず後始末する。
  useEffect(() => {
    setListenerPaused(true).catch(console.error);
    let cancelled = false;
    startMicTest(device).catch((e) => {
      if (!cancelled) setError(String(e));
    });
    return () => {
      cancelled = true;
      stopMicTest().catch(console.error);
      setListenerPaused(false).catch(console.error);
    };
  }, [device]);

  // 本番の audio-level とは別イベントで、テスト専用の録音レベルを受け取る。
  useEffect(() => {
    const promise = listen<number>("mic-test-level", (ev) => {
      levelRef.current = ev.payload;
    });
    return () => {
      promise.then((fn) => fn());
    };
  }, []);

  // 再レンダリングを避けるため、レベルバーの幅は ref 経由で直接 DOM を更新する
  // (StatusOverlay の Waveform と同じ方針)。
  useEffect(() => {
    if (!running) {
      if (barRef.current) barRef.current.style.width = "0%";
      return;
    }
    let raf = 0;
    const render = () => {
      const norm = Math.pow(Math.max(0, Math.min(1, levelRef.current / MIC_TEST_LEVEL_CEIL)), 0.4);
      if (barRef.current) barRef.current.style.width = `${Math.round(norm * 100)}%`;
      raf = requestAnimationFrame(render);
    };
    render();
    return () => cancelAnimationFrame(raf);
  }, [running]);

  const handleStop = () => {
    stopMicTest().catch(console.error);
    levelRef.current = 0;
    setRunning(false);
  };

  return (
    <Modal.Backdrop
      variant="blur"
      isOpen
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <Modal.Container>
        <Modal.Dialog className="sm:max-w-[420px]">
          <Modal.Header>
            <Modal.Heading>マイクをテスト</Modal.Heading>
            <p className="mt-1.5 text-sm text-muted">
              選択したマイクをテストするために話してください
            </p>
          </Modal.Header>
          <Modal.Body>
            <div className="rounded-xl border border-border bg-surface-secondary p-4">
              <div className="mb-2.5 flex items-center justify-between text-sm">
                <span className="font-medium text-foreground">
                  {device ?? "システムのデフォルト"}
                </span>
                {!error && (
                  <span className="text-xs font-medium text-success">
                    {running ? "アクティブ" : "停止中"}
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2">
                <IconMic className="size-4 shrink-0 text-muted" />
                <div className="h-2 flex-1 overflow-hidden rounded-full bg-surface-tertiary">
                  <div
                    ref={barRef}
                    className="h-full rounded-full bg-accent transition-[width] duration-75"
                    style={{ width: "0%" }}
                  />
                </div>
              </div>
            </div>
            {error ? (
              <Alert status="danger" className="mt-3">
                <Alert.Indicator />
                <Alert.Content>
                  <Alert.Description>{error}</Alert.Description>
                </Alert.Content>
              </Alert>
            ) : (
              <p className="mt-3 text-center text-xs text-muted">
                自分の声に反応してバーが動くはずです。動かない場合はマイクの権限を確認してください
              </p>
            )}
          </Modal.Body>
          <Modal.Footer>
            {running && (
              <Button variant="danger" onPress={handleStop}>
                停止
              </Button>
            )}
            <Button variant="secondary" onPress={onClose}>
              閉じる
            </Button>
          </Modal.Footer>
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}

// --------------- Models ---------------

const AUTH_KIND_OPTIONS = [
  { value: "bearer", label: "Bearer トークン" },
  { value: "api_key_header", label: "カスタムヘッダー" },
  { value: "none", label: "認証なし" },
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
  const headerName = config.auth_kind.kind === "api_key_header" ? config.auth_kind.header_name : "";

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
            <Input placeholder="例: whisper-1" />
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
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: (s: Settings) => void;
}) {
  return (
    <>
      <p className="mb-5 text-xs leading-relaxed text-muted">
        OpenAI 公式 / カスタムエンドポイント / ローカルなど OpenAI 互換 API に対応しています。
      </p>

      <ProviderSection
        label="STT (音声認識)"
        config={settings.stt}
        onChange={(stt) => onChange({ ...settings, stt })}
      />
    </>
  );
}

// --------------- Dictionary ---------------

function DictionaryPane({
  dict,
  onChange,
  flash,
}: {
  dict: Dictionary;
  onChange: (d: Dictionary) => void;
  flash: (type: "saved" | "error", text: string) => void;
}) {
  const add = () => onChange({ entries: [...dict.entries, { from: "", to: "" }] });
  const remove = (i: number) => onChange({ entries: dict.entries.filter((_, idx) => idx !== i) });
  const update = (i: number, field: "from" | "to", val: string) =>
    onChange({
      entries: dict.entries.map((e, idx) => (idx === i ? { ...e, [field]: val } : e)),
    });

  const handleImport = async () => {
    try {
      const result = await openFileDialog({
        multiple: false,
        filters: [{ name: "辞書ファイル (CSV / Dragon TXT)", extensions: ["csv", "txt"] }],
      });
      if (!result || Array.isArray(result)) return; // キャンセル
      const merged = await importDictionary(result);
      onChange(merged);
      flash("saved", `${merged.entries.length} 件をインポートしました`);
    } catch (e) {
      flash("error", String(e));
    }
  };

  const handleExport = async () => {
    try {
      const result = await saveFileDialog({
        defaultPath: "coatype-dictionary.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!result) return; // キャンセル
      await exportDictionary(result);
      flash("saved", "辞書を CSV にエクスポートしました");
    } catch (e) {
      flash("error", String(e));
    }
  };

  return (
    <>
      <p className="mb-5 text-xs leading-relaxed text-muted">
        文字起こし結果に対して辞書で置換します。CSV (RFC 4180) または Dragon TXT
        形式でインポートできます。
      </p>

      <Panel className="overflow-hidden">
        <div className="flex items-center gap-3 bg-surface-secondary px-4 py-2 text-xs font-semibold tracking-wide text-muted uppercase">
          <span className="flex-1">変換前 (from)</span>
          <span className="flex-1">変換後 (to)</span>
          <span className="w-8" />
        </div>
        {dict.entries.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-muted">
            エントリがありません。「+ 追加」またはファイルからインポートしてください。
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

      <div className="mt-4 flex flex-wrap justify-start gap-2">
        <Button variant="secondary" onPress={add}>
          <IconPlus className="size-4" />
          追加
        </Button>
        <Button variant="secondary" onPress={handleImport}>
          <IconUpload className="size-4" />
          インポート
        </Button>
        <Button variant="secondary" onPress={handleExport} isDisabled={dict.entries.length === 0}>
          <IconDownload className="size-4" />
          エクスポート
        </Button>
      </div>
    </>
  );
}

// --------------- History ---------------

function DictRegisterPopover({
  selectionText,
  rect,
  existingTo,
  onSubmit,
  onClose,
}: {
  selectionText: string;
  rect: DOMRect;
  existingTo?: string;
  onSubmit: (from: string, to: string) => void;
  onClose: () => void;
}) {
  const [from, setFrom] = useState(selectionText);
  const [to, setTo] = useState("");
  const [mode, setMode] = useState<"input" | "confirm">("input");
  const rootRef = useRef<HTMLDivElement>(null);

  const POPOVER_W = 284;
  const POPOVER_H = 168;
  const left = Math.min(Math.max(rect.left, 8), window.innerWidth - POPOVER_W - 8);
  const top = rect.top > POPOVER_H + 8 ? rect.top - POPOVER_H - 8 : rect.bottom + 8;

  useEffect(() => {
    // rAF 遅延で登録し、開いた直後の同一 mousedown で誤って閉じるのを防ぐ
    let removeListener: (() => void) | null = null;
    const rafId = requestAnimationFrame(() => {
      const onMouseDown = (e: MouseEvent) => {
        if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
          onClose();
        }
      };
      document.addEventListener("mousedown", onMouseDown);
      removeListener = () => document.removeEventListener("mousedown", onMouseDown);
    });
    return () => {
      cancelAnimationFrame(rafId);
      removeListener?.();
    };
  }, [onClose]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  const handleRegisterClick = () => {
    if (existingTo !== undefined && mode === "input") {
      setMode("confirm");
    } else {
      onSubmit(from.trim(), to.trim());
    }
  };

  return (
    <div
      ref={rootRef}
      data-dict-ui=""
      style={{ left, top, width: POPOVER_W }}
      className="fixed z-40 rounded-xl border border-border bg-surface shadow-[var(--surface-shadow)] p-3"
      onKeyDown={onKeyDown}
    >
      {mode === "input" ? (
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
            <IconBook className="size-3.5 text-accent" />
            辞書に登録
          </div>
          <TextField aria-label="変換前" className="w-full" value={from} onChange={setFrom}>
            <Input placeholder="変換前 (from)" />
          </TextField>
          <TextField aria-label="変換後" className="w-full" value={to} onChange={setTo} autoFocus>
            <Input placeholder="変換後 (to)" />
          </TextField>
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="secondary" onPress={onClose}>
              キャンセル
            </Button>
            <Button size="sm" onPress={handleRegisterClick} isDisabled={!from.trim() || !to.trim()}>
              登録
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          <div className="text-xs font-semibold text-foreground">上書きの確認</div>
          <p className="text-xs leading-relaxed text-muted">
            「{from}」は既に「{existingTo}」として登録されています。「{to}」で上書きしますか?
          </p>
          <div className="flex justify-end gap-2">
            <Button size="sm" variant="secondary" onPress={() => setMode("input")}>
              戻る
            </Button>
            <Button size="sm" variant="danger" onPress={() => onSubmit(from.trim(), to.trim())}>
              上書き
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

/** SQLite の "YYYY-MM-DD HH:MM:SS" (UTC) をローカル時刻の表記に整形する。パースできなければ原文のまま返す。 */
function formatCreatedAt(createdAt: string): string {
  const d = new Date(createdAt.replace(" ", "T") + "Z");
  if (Number.isNaN(d.getTime())) return createdAt;
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(d);
}

function HistoryPane({
  items,
  onClear,
  dict,
  onRegister,
}: {
  items: HistoryItem[];
  onClear: () => void;
  dict: Dictionary;
  onRegister: (from: string, to: string) => void;
}) {
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const { selection, selectionRef, clearSelection } = useTextSelection();
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [savedSelection, setSavedSelection] = useState<SelectionInfo | null>(null);

  const handleCopy = (id: number, text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    });
  };

  const handleDictButtonClick = () => {
    // mousedown 時に selectionchange が先行して state が null になる場合があるため ref を優先
    const current = selectionRef.current ?? selection;
    if (!current) return;
    setSavedSelection(current);
    setPopoverOpen(true);
  };

  const handlePopoverClose = () => {
    setPopoverOpen(false);
    setSavedSelection(null);
  };

  const handlePopoverSubmit = (from: string, to: string) => {
    onRegister(from, to);
    setPopoverOpen(false);
    setSavedSelection(null);
    clearSelection();
  };

  const FLOAT_BTN_H = 32;

  return (
    <>
      {/* 範囲選択時フローティングボタン */}
      {selection && !popoverOpen && (
        <button
          style={{
            position: "fixed",
            top: Math.min(
              Math.max(selection.rect.top - FLOAT_BTN_H - 4, 8),
              window.innerHeight - FLOAT_BTN_H - 8,
            ),
            left: Math.min(Math.max(selection.rect.left, 8), window.innerWidth - 148 - 8),
            zIndex: 40,
          }}
          data-dict-ui=""
          className="flex items-center gap-1.5 rounded-lg border border-border bg-surface px-2.5 py-1.5 text-xs font-medium text-foreground shadow-[var(--surface-shadow)] hover:bg-surface-secondary transition-colors"
          onMouseDown={(e) => {
            e.preventDefault(); // フォーカス移動抑止
            handleDictButtonClick();
          }}
        >
          <IconBook className="size-3.5 text-accent" />
          辞書に登録
        </button>
      )}

      {/* 辞書登録ポップオーバー */}
      {popoverOpen && savedSelection && (
        <DictRegisterPopover
          selectionText={savedSelection.text}
          rect={savedSelection.rect}
          existingTo={dict.entries.find((e) => e.from === savedSelection.text)?.to}
          onSubmit={handlePopoverSubmit}
          onClose={handlePopoverClose}
        />
      )}

      <div className="mb-4 flex w-full justify-end">
        <Button variant="danger" size="sm" onPress={onClear} isDisabled={items.length === 0}>
          全件削除
        </Button>
      </div>

      {items.length === 0 ? (
        <div className="w-full py-10 text-center text-sm text-muted">履歴がありません</div>
      ) : (
        <ul className="flex w-full flex-col gap-3">
          {items.map((item) => (
            <li key={item.id}>
              <Card variant="secondary" className="w-full border border-separator">
                <div className="flex items-end gap-2">
                  <div className="min-w-0 flex-1">
                    <div
                      className="leading-snug break-words text-foreground"
                      data-history-id={item.id}
                    >
                      {item.text}
                    </div>
                    <div className="mt-0.5 text-xs text-muted">
                      {formatCreatedAt(item.created_at)} · {item.language}
                      {item.translated ? " → 英語" : ""} · {item.duration_ms}ms
                    </div>
                  </div>
                  <div className="relative shrink-0">
                    {copiedId === item.id && (
                      <div className="pointer-events-none absolute bottom-full right-0 z-10 mb-2 whitespace-nowrap rounded-lg bg-foreground px-2.5 py-1 text-xs font-medium text-background shadow-lg">
                        コピーしました
                        <span className="absolute -bottom-1 right-3 size-2 rotate-45 bg-foreground" />
                      </div>
                    )}
                    <Button
                      isIconOnly
                      size="sm"
                      variant="outline"
                      onPress={() => handleCopy(item.id, item.text)}
                      aria-label="クリップボードにコピー"
                    >
                      <IconCopy className="size-4" />
                    </Button>
                  </div>
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
  apiKey,
  keyExists,
  onApiKeyChange,
  onSave,
}: {
  apiKey: string;
  keyExists: boolean;
  onApiKeyChange: (k: string) => void;
  onSave: (key: string) => void;
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
      <Section title="APIキー">
        <div className="flex flex-col gap-3">
          {keyExists && savedAlert("APIキーは macOS Keychain に保存済みです")}
          <Panel className="p-4">
            <TextField
              aria-label="APIキー"
              type="password"
              className="w-full"
              value={apiKey}
              onChange={onApiKeyChange}
            >
              <Input placeholder={keyExists ? "新しいキーで上書きする場合に入力…" : "sk-…"} />
            </TextField>
          </Panel>
          <div className="flex justify-end">
            <Button onPress={() => onSave(apiKey)} isDisabled={!apiKey.trim()}>
              キーを保存
            </Button>
          </div>
        </div>
      </Section>

      <Card variant="default" className="w-full">
        <p className="text-xs leading-relaxed text-muted">
          キーは macOS Keychain (
          <code className="rounded bg-surface-secondary px-1 py-0.5 font-mono text-[11px] text-foreground">
            jp.co.cotapon.coatype
          </code>
          ) に保存されます。
          <br />
          環境変数{" "}
          <code className="rounded bg-surface-secondary px-1 py-0.5 font-mono text-[11px] text-foreground">
            COATYPE_API_KEY
          </code>{" "}
          が設定されている場合はそちらが優先されます。
        </p>
      </Card>
    </>
  );
}
