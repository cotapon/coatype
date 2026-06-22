# CoAType — CLAUDE.md

AI エージェント向けのプロジェクト固有規約。グローバルの `~/.claude/CLAUDE.md` と併用する。

---

## セキュリティ制約 (絶対に変えないこと)

- **APIキーのハードコード禁止**: `bundled-key` feature は存在させない。APIキーは `COATYPE_API_KEY` 環境変数か macOS Keychain (`jp.co.cotapon.coatype`) のみ。
- Keychain の service name: `jp.co.cotapon.coatype`、username: `api-key`

---

## アーキテクチャ概要

```
src/                    # React (Vite) フロントエンド
  SettingsPage.tsx      # 設定UI (General / Dictionary / History / API Key)
  StatusOverlay.tsx     # 録音中オーバーレイ (220×60px, always-on-top)
  invoke.ts             # tauri invoke / listen ラッパー
  types.ts              # 共有型定義

src-tauri/src/
  main.rs               # Tauri セットアップ、shortcut → pipeline ループ
  commands.rs           # tauri::command 定義 (get/save settings, api_key 等)
  pipeline.rs           # 録音 → Whisper → 辞書補正 → 挿入 のオーケストレーター
  api/whisper.rs        # reqwest Whisper クライアント
  audio/recorder.rs     # cpal マイク録音 → WAV
  shortcut/listener.rs  # rdev grab によるグローバルショートカット監視
  injector.rs           # arboard + enigo でテキスト挿入 (Cmd+V)
  config/settings.rs    # Settings 構造体 (JSON永続化)
  dictionary/           # replace.rs (完全一致置換) + llm_correct.rs (LLM補正)
  history/store.rs      # SQLite 文字起こし履歴
  secrets/keychain.rs   # keyring による APIキー管理
  permissions.rs        # AXIsProcessTrustedWithOptions ラッパー
```

**Tauri ウィンドウ:**
- `settings` (720×560): 設定UI。× ボタンで閉じず非表示にする (再表示のため)
- `overlay` (220×60): 録音中オーバーレイ。always-on-top、transparent

**State 管理:**
- `Arc<Pipeline>` を `app.manage` — 全コマンドから共有
- `Arc<Mutex<ActiveShortcut>>` → `ListenerState` で manage — ショートカット状態表示用

---

## macOS 固有のハマりポイント

### CGEventTap と macOS 26 (Darwin 25.4+)

rdev のデフォルト実装は **HID レベル** のタップを使うが、macOS 26 からキーボードイベントが HID では配信されなくなった。

**修正箇所**: `src-tauri/vendor/rdev/src/macos/grab.rs`
```rust
// macOS 26 では HID でキーイベントが届かないため Session に変更
CGEventTapCreate(
    CGEventTapLocation::Session,  // 変更前: HID
    kCGHeadInsertEventTap,
    CGEventTapOption::Default,    // ListenOnly では Shift+Space が届かない
    ...
)
```

**使えないキー組み合わせ**: `Shift+Space` などの文字キーコンボは日本語 IME が Session タップより先に消費するため動作しない。UI の選択肢から除外済み (SettingsPage.tsx の `SHORTCUT_OPTIONS`)。

### rdev vendoring

`src-tauri/vendor/rdev/` にフォークを持つ。`Cargo.toml` の `[patch.crates-io]` で差し替え。

変更した箇所:
- `src/macos/mod.rs`: `grab` を `unstable_grab` feature gate なしで常時公開
- `src/macos/common.rs`: `CGEventTapOption::Default` の feature gate を除去
- `src/macos/grab.rs`: `CGEventTapLocation::Session` に変更
- `src/lib.rs`: `grab` のコールバック型を `Fn` → `FnMut` に変更 (クロージャ内でミュータブル状態を持てるように)
- `src/macos/keycodes.rs`: `key_from_code` に `CONTROL_RIGHT => Key::ControlRight` を追加 (欠落によりRightCtrlが `Key::Unknown(62)` になりバインド一致しなかった)

### keyring の apple-native feature

```toml
# Cargo.toml
keyring = { version = "3", features = ["apple-native"] }
```

`apple-native` feature を付けないと `MockCredential` (in-memory、非永続) になる。
`keyring::Entry::new().set_password()` が成功しているのに再起動後にキーが消えていたら、この feature が抜けている。

### WhisperClient のキー更新

`WhisperClient` は起動時に一度生成される。`save_api_key` コマンドで Keychain に保存しても古いインスタンスには反映されないため、`Arc<Mutex<String>>` で保持して `set_api_key()` + `pipeline.update_api_key()` で即時反映している。

```rust
// commands.rs save_api_key
secrets::keychain::save_api_key(&key)?;
pipeline.update_api_key(key);  // ← これがないと再起動まで 401 になる
```

### テキスト挿入

`arboard` でクリップボードにテキストをセット → `enigo` で Cmd+V を送信。
元のクリップボード内容は挿入後に復元する (`injector.rs`)。

### Dock / Cmd+Tab アイコンの更新

macOS の Dock と Cmd+Tab は、`.app` bundle の `Contents/Resources/icon.icns` を参照する。
このプロジェクトでは `src-tauri/icons/icon.icns` が元ファイルで、Tauri の bundle 生成時に `CoAType.app/Contents/Resources/icon.icns` へコピーされる。
メニューバー用の `src-tauri/icons/tray-icon.png` は Dock / Cmd+Tab には使われない。

アイコン変更時は、外側の余白を白背景ではなくアルファ 0 の透明ピクセルで作る。
見た目サイズを下げたい場合は、`icon_512x512@2x.png` 相当の 1024px キャンバスで白い角丸タイルの可視領域を 75〜80% 程度に収める。

Tauri dev では、アイコンだけを変更しても Cargo が既存の `target/debug/coatype` を再利用し、Dock / Cmd+Tab に古いアイコンが残ることがある。
その場合は macOS キャッシュ削除ではなく、まず CoAType のビルド成果物を消して再ビルドさせる。

```bash
lsof -ti tcp:1420 | xargs kill
cargo clean -p coatype --manifest-path src-tauri/Cargo.toml
npm run dev:tauri
```

`cargo clean -p coatype --manifest-path src-tauri/Cargo.toml` は macOS の IconServices キャッシュを消さない。
消すのは Cargo の `target` 配下のビルド成果物で、次回 `npm run dev:tauri` 時に新しい `src-tauri/icons/icon.icns` を取り込ませるために使う。
再ビルド後も Dock 表示が古い場合のみ `killall Dock` で Dock の表示キャッシュを読み直す。

---

## 開発フロー

```bash
npm run tauri dev   # 開発サーバー (Rust + Vite のホットリロード)
npm test            # ユニットテスト (cargo test --manifest-path src-tauri/Cargo.toml)
```

初回 `cargo build` は依存クレートが多く 5〜10 分かかる。

### ショートカットのデバッグ

Settings の General ペインに「現在有効: <値> / 状態: ✓ 有効 | ❌」バッジを表示。
tracing ログ (`RUST_LOG=debug npm run tauri dev`) でキーイベントを確認できる。

---

## 実装上の制約・方針

- `save_settings` はリスナーを再起動しない。ショートカットキーの変更はアプリ再起動で反映される (将来的に `CFRunLoopStop` + 再 spawn で対応可能だが現状は未実装)
- LLM補正クライアントは `main.rs` で `None` を渡している (Task 11 以降で初期化予定)
- `cargo test` はすべて mockito を使った HTTP モックテスト。実際の API を叩くテストはない
