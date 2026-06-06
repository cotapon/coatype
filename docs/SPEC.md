# CoAType 技術仕様書

**Voice Typing for CyberAgent** — Powered by whisper-large-v3

---

## 目次

1. [概要](#概要)
2. [アーキテクチャ](#アーキテクチャ)
3. [機能仕様](#機能仕様)
4. [Tauri コマンド API](#tauri-コマンド-api)
5. [音声録音パイプライン](#音声録音パイプライン)
6. [Whisper API クライアント](#whisper-api-クライアント)
7. [テキスト挿入 (Injector)](#テキスト挿入-injector)
8. [ショートカットキーシステム](#ショートカットキーシステム)
9. [フォーカス管理](#フォーカス管理)
10. [設定・永続化](#設定永続化)
11. [APIキー管理](#apiキー管理)
12. [辞書・LLM補正](#辞書llm補正)
13. [文字起こし履歴](#文字起こし履歴)
14. [macOS 26 対応](#macos-26-対応)
15. [実装中に解決したバグ](#実装中に解決したバグ)
16. [既知の制約](#既知の制約)
17. [依存クレート](#依存クレート)
18. [ビルド・開発フロー](#ビルド開発フロー)

---

## 概要

macOS 向けメニューバーアプリ。ショートカットキーを押している間（Push-to-Talk）またはトグル操作でマイクから音声を録音し、CyberAgent ML プラットフォームの Whisper API でテキストに変換してアクティブなアプリケーションに直接入力する。

| 項目 | 値 |
|---|---|
| アプリ名 | CoAType (コエタイプ) |
| Bundle ID | `jp.co.cyberagent.coatype` |
| プラットフォーム | macOS 13.0 (Ventura) 以降 |
| フレームワーク | Tauri v2 |
| フロントエンド | React 18 + Vite |
| バックエンド | Rust (stable) |
| Whisper エンドポイント | `https://genai.mlplatform.apis.platform.cycloud.jp` |
| Whisper モデル | `whisper-large-v3` |

---

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│  macOS                                                    │
│  ┌──────────────┐   ┌────────────────────────────────┐  │
│  │  settings    │   │  overlay (220×60, always-on-top│  │
│  │  window      │   │  transparent, no decoration)   │  │
│  │  (720×560)   │   └────────────────────────────────┘  │
│  │  React SPA   │                                        │
│  └──────┬───────┘                                        │
│         │ Tauri invoke / events                           │
│  ┌──────▼──────────────────────────────────────────────┐ │
│  │  Rust バックエンド (src-tauri/src/)                  │ │
│  │                                                      │ │
│  │  listener.rs ──(ShortcutEvent)──► main.rs            │ │
│  │  (rdev grab)       mpsc          ├─► pipeline.rs     │ │
│  │                                  │   ├─ recorder.rs  │ │
│  │  focus.rs ◄── main.rs ──────────►│   ├─ whisper.rs   │ │
│  │  (NSWorkspace)                   │   ├─ dictionary   │ │
│  │                                  │   └─ history      │ │
│  │  injector/ ◄── main thread ◄─────┘                   │ │
│  │  (arboard+enigo)                                     │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                            │
│  System: CGEventTap (Session) / Keychain / SQLite         │
└─────────────────────────────────────────────────────────┘
         │ HTTPS multipart/form-data
         ▼
  CyberAgent ML Platform (Whisper API)
```

### ファイル構成

```
coatype/
├── src/                        # React フロントエンド
│   ├── App.tsx                 # ルートコンポーネント (ウィンドウ切り替え)
│   ├── SettingsPage.tsx        # 設定UI (General/Dictionary/History/API Key タブ)
│   ├── StatusOverlay.tsx       # 録音中インジケーター
│   ├── invoke.ts               # Tauri invoke/listen ラッパー
│   └── types.ts                # 共有型定義
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # エントリポイント・イベントループ
│   │   ├── lib.rs              # クレートルート
│   │   ├── commands.rs         # tauri::command 定義
│   │   ├── pipeline.rs         # 録音→文字起こし→補正→挿入 オーケストレーター
│   │   ├── focus.rs            # macOS フォーカス管理
│   │   ├── permissions.rs      # Accessibility 権限チェック
│   │   ├── injector/
│   │   │   ├── mod.rs
│   │   │   └── macos.rs        # arboard + enigo による Cmd+V 挿入
│   │   ├── api/
│   │   │   ├── whisper.rs      # reqwest Whisper クライアント
│   │   │   └── error.rs        # API エラー型
│   │   ├── audio/
│   │   │   └── recorder.rs     # cpal マイク録音 → WAV
│   │   ├── shortcut/
│   │   │   └── listener.rs     # rdev grab グローバルショートカット
│   │   ├── config/
│   │   │   └── settings.rs     # Settings 構造体 (JSON 永続化)
│   │   ├── dictionary/
│   │   │   ├── replace.rs      # 完全一致文字列置換
│   │   │   └── llm_correct.rs  # LLM 辞書補正クライアント
│   │   ├── history/
│   │   │   └── store.rs        # SQLite 文字起こし履歴
│   │   └── secrets/
│   │       └── keychain.rs     # keyring による APIキー管理
│   ├── vendor/rdev/            # フォーク版 rdev (macOS 26 対応パッチ済み)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── Info.plist
└── docs/
    └── SPEC.md                 # 本文書
```

---

## 機能仕様

### ショートカットキー

| 設定値 | 説明 |
|---|---|
| `rightoption` (デフォルト) | 右 Option キー |
| `leftoption` | 左 Option キー |
| `rightcontrol` | 右 Control キー |
| `leftcontrol` | 左 Control キー |
| `f5` 〜 `f8` | ファンクションキー |

コンボキー (`leftshift+space` など) は macOS 26 では IME に消費されるため **UI 選択肢から除外**している。

### トリガーモード

| モード | 動作 |
|---|---|
| Push-to-Talk | キー押下中のみ録音。離すと文字起こし開始 |
| Toggle | 1 回押すと録音開始、もう 1 回で停止 |

### 文字起こし言語

`language` 設定 (BCP-47 言語コード: `ja`, `en`, `zh` 等) を Whisper API の `language` パラメーターに渡す。

### 英語翻訳モード

`translate_mode: true` のとき、`/v1/audio/transcriptions` の代わりに `/v1/audio/translations` を使い、音声を英語に翻訳して返す。

### カスタム辞書

完全一致置換 (`from` → `to`) を毎回適用。エントリは設定 UI の Dictionary タブで管理し `dictionary.json` に保存。

### LLM 補正 (実験的)

`llm_correct: true` かつ `LlmCorrectClient` が初期化されているとき、辞書置換後のテキストを LLM に渡して文脈補正を行う。現状 `main.rs` では `None` を渡しているため無効状態。

### テキスト挿入

クリップボード経由 (arboard で set → enigo で Cmd+V)。挿入後に元のクリップボード内容を復元する。日本語・絵文字を含む全文字に対応。

---

## Tauri コマンド API

すべて `invoke()` で呼び出す非同期コマンド。

| コマンド | 引数 | 戻り値 | 説明 |
|---|---|---|---|
| `get_settings` | — | `Settings` | 設定ファイルを読み込んで返す |
| `save_settings` | `settings: Settings` | `void` | 設定を保存し、pipeline の in-memory 値を即時更新 |
| `get_dictionary` | — | `Dictionary` | pipeline の現在の辞書を返す |
| `save_dictionary` | `dict: Dictionary` | `void` | 辞書ファイルを保存し pipeline を更新 |
| `list_history` | `limit: i64` | `HistoryItem[]` | 文字起こし履歴を新しい順に最大 limit 件返す |
| `clear_history` | — | `void` | 履歴を全件削除 |
| `save_api_key` | `key: String` | `void` | Keychain に保存 + pipeline の WhisperClient を即時更新 |
| `has_api_key` | — | `bool` | Keychain または環境変数にキーがあるか |
| `check_accessibility` | — | `bool` | Accessibility 権限の有無を返す |
| `open_accessibility_settings` | — | `void` | システム設定の Accessibility ページを開く |
| `active_shortcut` | — | `ActiveShortcut` | 現在動作中のショートカット状態を返す |

### Tauri イベント (Rust → フロントエンド)

| イベント名 | ペイロード | タイミング |
|---|---|---|
| `recording-state` | `"started" \| "processing" \| "idle"` | 録音状態変化時 |
| `transcribed` | `String` (文字起こし結果) | pipeline 処理完了時 |
| `error` | `String` (エラーメッセージ) | 録音開始失敗・API エラー時 |
| `shortcut-error` | `String` | ショートカット解析失敗・EventTap 作成失敗時 |

### 型定義

```typescript
interface Settings {
  language: string;           // BCP-47 言語コード
  shortcut: string;           // "rightoption" | "leftcontrol" | ...
  trigger_mode: "push_to_talk" | "toggle";
  translate_mode: boolean;
  llm_correct: boolean;
  api_base: string;           // Whisper API ベース URL
}

interface ActiveShortcut {
  shortcut: string;
  trigger_mode: string;
  status: "starting" | "ok" | "parse_error" | "tap_failed";
  error: string | null;
}

interface HistoryItem {
  id: number;
  text: string;
  language: string;
  translated: boolean;
  duration_ms: number;
  created_at: string;         // ISO 8601
}
```

---

## 音声録音パイプライン

### Recorder (`audio/recorder.rs`)

1. `cpal::default_host().default_input_device()` でデフォルトマイクを取得
2. デバイスのデフォルト入力設定 (`sample_rate`, `sample_format`) を使用
3. f32 サンプル → i16 に変換して内部バッファに蓄積
4. `stop()` 時に `hound::WavWriter` で 16bit mono WAV にエンコードして返す

### 無音検出 (`pipeline.rs::wav_is_silent`)

WAV ヘッダー (44 bytes) をスキップし、i16 サンプルの RMS を計算。

```
RMS = sqrt(Σ(s²) / N)
```

**閾値**: 300 (i16::MAX の約 0.9%)  
RMS < 300 の場合は Whisper API を呼ばずに空文字を返す。

> Whisper は無音・環境音のみの音声に対して「ありがとうございます」「ありがとうございました」等を幻覚する。この閾値でフィルタリングすることで誤挿入を防ぐ。

---

## Whisper API クライアント

### エンドポイント

| 用途 | パス |
|---|---|
| 文字起こし | `POST /v1/audio/transcriptions` |
| 英語翻訳 | `POST /v1/audio/translations` |

### リクエスト形式

`multipart/form-data`

| フィールド | 値 |
|---|---|
| `file` | `audio.wav` (audio/wav) |
| `model` | `whisper-large-v3` |
| `language` | BCP-47 言語コード (transcriptions のみ) |

### レスポンス

```json
{ "text": "文字起こし結果" }
```

### APIキー更新の仕組み

`WhisperClient` は起動時に 1 度生成される。後から `save_api_key` で Keychain に保存しても、古いインスタンスには反映されない問題を `Arc<Mutex<String>>` で解決:

```rust
pub struct WhisperClient {
    api_key: Arc<Mutex<String>>,
    // ...
}
pub fn set_api_key(&self, key: String) {
    *self.api_key.lock().unwrap() = key;
}
```

`save_api_key` コマンドは Keychain 保存と `pipeline.update_api_key()` を必ず両方呼ぶ。

---

## テキスト挿入 (Injector)

### `injector/macos.rs`

1. `arboard::Clipboard::set_text(text)` でクリップボードにテキストをセット
2. 50ms 待機 (クリップボード反映待ち)
3. `enigo` で `Meta(Press)` → `Unicode('v', Click)` → `Meta(Release)` を送信 (= Cmd+V)
4. 100ms 待機 (ペースト完了待ち)
5. 元のクリップボード内容を復元

### スレッド制約

`enigo` は内部で macOS Text Services Manager (TSM) の `TSMGetInputSourceProperty` を呼ぶ。TSM は **main thread でのみ呼び出し可能**。tokio worker thread から呼ぶと `dispatch_assert_queue_fail` で `EXC_BREAKPOINT (SIGTRAP)` クラッシュが発生する。

**対策**: `pipeline.stop_and_process()` の中には `injector::insert` を置かず、`main.rs` の `run_on_main_thread` クロージャ内でのみ呼び出す。

```rust
// main.rs
handle.run_on_main_thread(move || {
    coatype_lib::focus::restore_frontmost(prev_pid);
    std::thread::sleep(Duration::from_millis(80));
    coatype_lib::injector::insert(&text).ok();
})?;
```

---

## ショートカットキーシステム

### rdev グローバルキー監視

`vendor/rdev` (フォーク) の `grab()` 関数で `CGEventTap` を作成し、システム全体のキーイベントを傍受する。コールバック内で `ShortcutEvent::{StartRecording, StopRecording}` を mpsc チャネルに送信。

### Push-to-Talk ロジック

```
KeyPress(target) かつ !is_held  →  is_held = true, StartRecording 送信
KeyRelease(target)              →  is_held = false, StopRecording 送信
```

`is_held` フラグでキーリピートによる誤発火を防止。

### FlagsChanged バグ修正 (macOS 26)

修飾キーのイベントは `CGEventType::FlagsChanged` として届く。**旧実装の問題**:

```rust
// 問題: flags < LAST_FLAGS はビットマスクの数値比較
// Right Option を押しながら CapsLock が変化すると疑似 KeyRelease が発火
if flags < LAST_FLAGS { KeyRelease } else { KeyPress }
```

**修正後** (`vendor/rdev/src/macos/common.rs`):

```rust
// そのキーに対応するフラグビットが今回のイベントで落ちたか否かで判定
let bit = flag_bit_for_keycode(code);  // 例: AltGr(61) → 0x80000
let is_release = if bit != 0 {
    (LAST_FLAGS.bits() & bit) != 0 && (flags.bits() & bit) == 0
} else {
    flags < LAST_FLAGS  // フォールバック
};
```

`flag_bit_for_keycode` マッピング:

| keycode | キー | CGEventFlag bit |
|---|---|---|
| 56, 60 | Shift L/R | 0x020000 |
| 59, 62 | Control L/R | 0x040000 |
| 58, 61 | Option L/R | 0x080000 |
| 54, 55 | Command L/R | 0x100000 |
| 57 | CapsLock | 0x010000 |
| 63 | Fn | 0x800000 |

---

## フォーカス管理

### 問題

Tauri の `WebviewWindow::show()` は内部で `[NSWindow makeKeyAndOrderFront:]` を呼ぶため、overlay が表示されるとフォーカスが CoAType に移動し、その後の Cmd+V が本来の入力先に届かない。

### 解決策 (`src/focus.rs`)

| 関数 | 処理 |
|---|---|
| `capture_frontmost_pid() -> Option<i32>` | `NSWorkspace.frontmostApplication.processIdentifier` で前アプリの PID を取得 |
| `restore_frontmost(pid: i32)` | `NSRunningApplication.runningApplicationWithProcessIdentifier(pid).activateWithOptions(...)` で前アプリを再アクティベート |
| `show_panel(ns_window: *mut c_void)` | `[NSWindow orderFront:nil]` でフォーカスを奪わずに overlay を表示 |

### 挿入前フォーカス復元フロー

```
1. KeyPress  → capture_frontmost_pid() → 前アプリ PID 保存
             → run_on_main_thread { show_panel(overlay) }  // フォーカス奪取なし

2. KeyRelease → stop_and_process()
             → run_on_main_thread {
                 restore_frontmost(prev_pid)   // 前アプリを前面に
                 sleep(80ms)                    // フォーカス切替待ち
                 injector::insert(text)         // Cmd+V → 前アプリに入力
               }
```

### アプリ起動時のフォーカス奪取防止

`Info.plist` に `<key>LSUIElement</key><true/>` を設定。これにより:
- アプリが Dock に表示されない
- 起動時にフォーカスが移動しない
- ユーザーが明示的に設定ウィンドウをクリックした場合は通常通りフォーカスを受け取る

---

## 設定・永続化

### 設定ファイル

パス: `~/Library/Application Support/jp.co.cyberagent.coatype/settings.json`

```json
{
  "language": "ja",
  "shortcut": "rightoption",
  "trigger_mode": "push_to_talk",
  "translate_mode": false,
  "llm_correct": false,
  "api_base": "https://genai.mlplatform.apis.platform.cycloud.jp"
}
```

ファイルが存在しない・パース失敗の場合は `Settings::default()` を使用。

### 辞書ファイル

パス: `~/Library/Application Support/jp.co.cyberagent.coatype/dictionary.json`

```json
{
  "entries": [
    { "from": "まちがい", "to": "正解" }
  ]
}
```

### 文字起こし履歴

パス: `~/Library/Application Support/jp.co.cyberagent.coatype/history.db` (SQLite)

スキーマ:

```sql
CREATE TABLE history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    text        TEXT NOT NULL,
    language    TEXT NOT NULL,
    translated  INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

---

## APIキー管理

### セキュリティ要件

- **ハードコード禁止**: `bundled-key` feature は存在させない
- 有効なソースは環境変数 `COATYPE_API_KEY` または macOS Keychain のみ

### Keychain エントリ

| 項目 | 値 |
|---|---|
| Service | `jp.co.cyberagent.coatype` |
| Account | `api-key` |
| crate | `keyring = { version = "3", features = ["apple-native"] }` |

`apple-native` feature が必須。省略すると `MockCredential` (in-memory) になり再起動後にキーが消える。

### 優先順位

1. 環境変数 `COATYPE_API_KEY` (空でない場合)
2. macOS Keychain

---

## 辞書・LLM補正

### 完全一致置換 (`dictionary/replace.rs`)

`Dictionary::apply(text)` が毎回実行される。エントリの `from` が `text` に含まれれば `to` に置換。複数エントリは定義順に適用。

### LLM 補正 (`dictionary/llm_correct.rs`)

`llm_correct: true` かつ `LlmCorrectClient` が Some のとき、辞書置換後のテキストと辞書エントリリストを LLM に渡して文脈を考慮した補正を実施。現状 `main.rs` では `None` を渡しているため無効。

---

## 文字起こし履歴

`HistoryStore::insert` は各文字起こし完了時に呼ばれる。保存されるフィールド:

- `text`: 最終テキスト (辞書・LLM 補正後)
- `language`: 使用言語コード
- `translated`: 翻訳モードだったか否か
- `duration_ms`: 録音開始から API 応答完了までのミリ秒
- `created_at`: UTC タイムスタンプ

---

## macOS 26 対応

macOS 26 (Darwin 25.4 / Sequoia 26) から CGEventTap の動作が変更された。

### CGEventTap 配信レベルの変更

| レベル | macOS 25 以前 | macOS 26 以降 |
|---|---|---|
| HID (デフォルト) | キーイベント届く | **キーイベントが届かない** |
| Session | キーイベント届く | キーイベント届く |

**対処** (`vendor/rdev/src/macos/grab.rs`):

```rust
CGEventTapCreate(
    CGEventTapLocation::Session,  // 変更前: HID
    kCGHeadInsertEventTap,
    CGEventTapOption::Default,    // ListenOnly では Shift+Space が届かない
    ...
)
```

### 使用できないキー組み合わせ

日本語 IME が Session レベルより先に `Shift+Space` などの文字キーを消費するため、ショートカットとして設定できるキーは**修飾キー単体のみ**。

---

## 実装中に解決したバグ

### 1. TSM main-thread クラッシュ

**症状**: 文字起こし後に `EXC_BREAKPOINT (SIGTRAP)` でアプリがクラッシュ

**原因**: `enigo` が内部で `TSMGetInputSourceProperty` (HIToolbox) を呼ぶが、これは main thread 専用 API。`pipeline.stop_and_process()` は tokio worker thread で実行されていたため `dispatch_assert_queue_fail` が発生。

**修正**: `injector::insert` の呼び出しを `pipeline.rs` から削除し、`main.rs` の `handle.run_on_main_thread()` クロージャに移動。

---

### 2. Push-to-Talk 長押し中に即停止

**症状**: キーを長押ししているにもかかわらず、すぐに processing 状態になり前回の文字起こし結果が挿入される

**原因**: `vendor/rdev/src/macos/common.rs` の `FlagsChanged` 処理:

```rust
// 旧: フラグビットマスクを数値として比較
if flags < LAST_FLAGS { KeyRelease } else { KeyPress }
```

Right Option 押下中に CapsLock 等の別の修飾キーが変化すると `flags` の数値が `LAST_FLAGS` より小さくなり、疑似 `KeyRelease` が誤発火していた。

**修正**: 当該キーに対応する特定のフラグビットが消えたかどうかで KeyRelease を判定する方式に変更。

---

### 3. 起動時・録音中のフォーカス奪取

**症状**: アプリ起動時またはオーバーレイ表示時にフォーカスが CoAType に移り、Cmd+V が入力対象に届かない

**原因**:
- アプリ起動時: macOS デフォルトの activation policy でフォーカスが移動
- オーバーレイ表示時: `w.show()` が内部で `makeKeyAndOrderFront` を呼ぶ

**修正**:
1. `Info.plist` に `LSUIElement = YES` を追加 (起動時の activation を抑制)
2. overlay 表示を `[NSWindow orderFront:]` に変更 (フォーカス奪取なし)
3. 注入前に `NSRunningApplication.activateWithOptions` で前アプリを復元

---

### 4. APIキー保存後も 401 エラー

**症状**: Settings で API キーを保存しても、アプリを再起動するまで 401 エラーが続く

**原因**: `WhisperClient` は起動時に 1 度生成され、以後変更されない

**修正**: `api_key` フィールドを `Arc<Mutex<String>>` で保持し、`save_api_key` コマンドで Keychain 保存と同時に `pipeline.update_api_key()` を呼んで in-memory 値を即時更新。

---

### 5. Whisper の無音幻覚

**症状**: 無言でキーを押すと「ありがとうございます」等が挿入される

**原因**: Whisper が音声信号なしのとき学習データに多い定型フレーズを出力する (hallucination)

**修正**: 録音した WAV の RMS が 300 未満の場合は API を呼ばずに空文字を返す。

---

## 既知の制約

| 制約 | 詳細 |
|---|---|
| ショートカットキーは修飾キー単体のみ | macOS 26 で `Shift+Space` 等の文字キーコンボは IME に消費される |
| クリップボード内容が一時上書きされる | 挿入後に復元するが、競合するとまれに失われる |
| ペーストを受け付けないアプリは動作しない | 一部ターミナル、セキュリティ入力フィールド等 |
| LLM 補正は未初期化 | `main.rs` で `None` を渡しており、設定 ON でも動作しない |
| ショートカット変更はアプリ再起動が必要 | `save_settings` はリスナーを再起動しない (将来 `CFRunLoopStop` + 再 spawn で対応可能) |

---

## 依存クレート

### バックエンド主要クレート

| クレート | バージョン | 用途 |
|---|---|---|
| `tauri` | 2.x | アプリフレームワーク (tray, macos-private-api) |
| `cpal` | 0.15 | クロスプラットフォーム音声入出力 |
| `hound` | 3.5 | WAV エンコード |
| `rdev` | 0.5 (vendor) | グローバルキーイベント監視 (CGEventTap) |
| `enigo` | 0.2 | キーボード入力シミュレーション |
| `arboard` | 3 | クリップボード操作 |
| `reqwest` | 0.12 | HTTP クライアント (rustls-tls) |
| `keyring` | 3 (apple-native) | macOS Keychain |
| `rusqlite` | 0.31 (bundled) | SQLite (履歴ストア) |
| `serde_json` | 1 | 設定ファイル JSON |
| `objc2-app-kit` | 0.3 | NSWorkspace / NSRunningApplication / NSWindow |
| `tokio` | 1 (full) | 非同期ランタイム |

### フロントエンド主要パッケージ

| パッケージ | 用途 |
|---|---|
| `@tauri-apps/api` | Tauri invoke / events |
| `react` 18 | UI フレームワーク |
| `vite` | バンドラー |

---

## ビルド・開発フロー

### 開発サーバー起動

```bash
npm run tauri dev
```

初回は Rust のコンパイルに 5〜10 分かかる。

### テスト

```bash
cd src-tauri
cargo test
```

テストはすべて `mockito` を使った HTTP モック。実 API は叩かない。

### 本番ビルド

```bash
npm run tauri build
# 成果物: src-tauri/target/release/bundle/macos/CoAType.app
```

### デバッグ

ショートカットキーイベントのトレース:

```bash
RUST_LOG=debug npm run tauri dev
```

### macOS 権限

初回起動時に必要:
1. **マイク**: システム設定 → プライバシーとセキュリティ → マイク → CoAType を許可
2. **アクセシビリティ**: システム設定 → プライバシーとセキュリティ → アクセシビリティ → `+` で CoAType を追加してチェック ON

---

## セキュリティ制約 (変更禁止)

- **APIキーのハードコード絶対禁止**: `COATYPE_API_KEY` 環境変数または macOS Keychain のみ
- `bundled-key` feature は追加しない
- Keychain service: `jp.co.cyberagent.coatype` / account: `api-key`
