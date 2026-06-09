# オーバーレイ表示トグル & 位置変更 実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 設定画面にオーバーレイ表示ON/OFFトグルを追加し、オーバーレイの表示位置を画面下部中央（下端40px）に変更する

**Architecture:** `Settings` 構造体に `show_overlay: bool` を追加し、既存の `ListenerPaused(Arc<AtomicBool>)` パターンと同じく `ShowOverlay(Arc<AtomicBool>)` としてランタイム状態を管理する。`show_overlay_panel` では表示前にフラグをチェックし、`window.current_monitor()` でスクリーンサイズを取得して底部中央に位置計算・設定する。

**Tech Stack:** Rust / Tauri v2 (`tauri::PhysicalPosition`, `tauri::Monitor`), React / TypeScript

---

## 変更ファイル一覧

| ファイル | 変更内容 |
|---|---|
| `src-tauri/src/config/settings.rs` | `show_overlay: bool` フィールド追加、`default_true()` 追加、`Default` 実装更新 |
| `src-tauri/src/commands.rs` | `ShowOverlay(pub Arc<AtomicBool>)` struct 追加、`save_settings` で AtomicBool を更新 |
| `src-tauri/src/main.rs` | `ShowOverlay` を manage・初期化、`show_overlay_panel` に位置計算とフラグチェックを追加 |
| `src/types.ts` | `Settings` interface に `show_overlay: boolean` を追加 |
| `src/SettingsPage.tsx` | `GeneralPane` の「オプション」セクションにチェックボックスを追加 |

---

### Task 1: settings.rs に `show_overlay` フィールドを追加

**Files:**
- Modify: `src-tauri/src/config/settings.rs`

- [ ] **Step 1: テストを書く**

`src-tauri/src/config/settings.rs` の `#[cfg(test)] mod tests` ブロックの末尾に追加：

```rust
#[test]
fn show_overlay_defaults_to_true_when_missing() {
    let json = r#"{
        "language": "ja",
        "bindings": [],
        "translate_mode": false,
        "llm_correct": false,
        "stt": {"base_url": "https://example.com", "model": "m", "auth_kind": {"kind": "bearer"}},
        "llm": {"base_url": "https://example.com", "model": "m", "auth_kind": {"kind": "bearer"}},
        "separate_api_keys": false
    }"#;
    let s: Settings = serde_json::from_str(json).unwrap();
    assert!(s.show_overlay, "show_overlay が JSON にない場合は true がデフォルト");
}

#[test]
fn show_overlay_false_roundtrip() {
    let mut s = Settings::default();
    s.show_overlay = false;
    let json = serde_json::to_string(&s).unwrap();
    let parsed: Settings = serde_json::from_str(&json).unwrap();
    assert!(!parsed.show_overlay);
}
```

- [ ] **Step 2: テストが失敗することを確認する**

```bash
cargo test --manifest-path src-tauri/Cargo.toml show_overlay 2>&1 | tail -20
```

期待出力: `error[E0609]: no field 'show_overlay'` または `FAILED`

- [ ] **Step 3: `default_true` 関数と `show_overlay` フィールドを追加する**

`src-tauri/src/config/settings.rs` の `Settings` 構造体（`separate_api_keys` フィールドの直後）に追加：

```rust
    #[serde(default = "default_true")]
    pub show_overlay: bool,
```

同ファイルの `impl Default for Settings` 内の `separate_api_keys: false,` の直後に追加：

```rust
            show_overlay: true,
```

同ファイルの `impl Settings` の外 (例: `impl Default` の直前あたり) にヘルパー関数を追加：

```rust
fn default_true() -> bool {
    true
}
```

- [ ] **Step 4: テストが通ることを確認する**

```bash
cargo test --manifest-path src-tauri/Cargo.toml show_overlay 2>&1 | tail -10
```

期待出力:
```
test config::settings::tests::show_overlay_defaults_to_true_when_missing ... ok
test config::settings::tests::show_overlay_false_roundtrip ... ok
```

- [ ] **Step 5: 既存のテスト全体が通ることを確認する**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

期待出力: `test result: ok. N passed`

- [ ] **Step 6: コミット**

```bash
git add src-tauri/src/config/settings.rs
git commit -m "feat(settings): show_overlay フィールドを追加 (デフォルト: true)"
```

---

### Task 2: types.ts に `show_overlay` を追加

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: `Settings` interface に追加する**

`src/types.ts` の `Settings` interface 内、`separate_api_keys: boolean;` の直後に追加：

```typescript
  show_overlay: boolean;
```

変更後の `Settings` interface:
```typescript
export interface Settings {
  language: string;
  bindings: KeyBinding[];
  translate_mode: boolean;
  llm_correct: boolean;
  stt: ProviderConfig;
  llm: ProviderConfig;
  separate_api_keys: boolean;
  show_overlay: boolean;
}
```

- [ ] **Step 2: コミット**

```bash
git add src/types.ts
git commit -m "feat(types): Settings に show_overlay を追加"
```

---

### Task 3: commands.rs に `ShowOverlay` state を追加し、save_settings で更新

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: `ShowOverlay` struct を追加する**

`src-tauri/src/commands.rs` の既存の struct 定義群（`ListenerPaused` の直後）に追加：

```rust
pub struct ShowOverlay(pub Arc<AtomicBool>);
```

変更後のブロック:
```rust
pub struct ListenerState(pub Arc<Mutex<ActiveShortcut>>);
pub struct ListenerBindings(pub Arc<Mutex<Vec<RegisteredBinding>>>);
pub struct ListenerPaused(pub Arc<AtomicBool>);
pub struct ShowOverlay(pub Arc<AtomicBool>);

pub struct SettingsPath(pub PathBuf);
pub struct DictPath(pub PathBuf);
```

- [ ] **Step 2: `save_settings` のシグネチャに `ShowOverlay` を追加し、保存時に更新する**

`src-tauri/src/commands.rs` の `save_settings` 関数を以下のように変更する：

変更前:
```rust
#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    path: State<'_, SettingsPath>,
    pipeline: State<'_, Arc<Pipeline>>,
    bindings_state: State<'_, ListenerBindings>,
) -> Result<(), String> {
    settings.save(&path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &**pipeline;
    *p.translate.lock().unwrap() = settings.translate_mode;
    *p.language.lock().unwrap() = settings.language.clone();
    *p.llm_correct.lock().unwrap() = settings.llm_correct;

    let stt_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_STT } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    let llm_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_LLM } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    p.rebuild_stt_client(&settings.stt, stt_key);
    p.rebuild_llm_client(&settings.llm, llm_key);

    // キーバインドをホットリロード (リスナー再起動なし)
    let new_registered = bindings_to_registered(&settings.bindings);
    *bindings_state.0.lock().unwrap() = new_registered;

    Ok(())
}
```

変更後:
```rust
#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    path: State<'_, SettingsPath>,
    pipeline: State<'_, Arc<Pipeline>>,
    bindings_state: State<'_, ListenerBindings>,
    show_overlay_state: State<'_, ShowOverlay>,
) -> Result<(), String> {
    settings.save(&path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &**pipeline;
    *p.translate.lock().unwrap() = settings.translate_mode;
    *p.language.lock().unwrap() = settings.language.clone();
    *p.llm_correct.lock().unwrap() = settings.llm_correct;

    let stt_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_STT } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    let llm_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_LLM } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    p.rebuild_stt_client(&settings.stt, stt_key);
    p.rebuild_llm_client(&settings.llm, llm_key);

    // キーバインドをホットリロード (リスナー再起動なし)
    let new_registered = bindings_to_registered(&settings.bindings);
    *bindings_state.0.lock().unwrap() = new_registered;

    show_overlay_state.0.store(settings.show_overlay, Ordering::Relaxed);

    Ok(())
}
```

- [ ] **Step 3: ビルドが通ることを確認する**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -10
```

期待出力: (空 — エラーなし)

- [ ] **Step 4: コミット**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): ShowOverlay state を追加し save_settings で更新"
```

---

### Task 4: main.rs で ShowOverlay を管理し、show_overlay_panel を更新

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: `ShowOverlay` を import に追加する**

`src-tauri/src/main.rs` の既存の import 行を変更する。

変更前:
```rust
use coatype_lib::commands::{
    ActiveShortcut, DictPath, ListenerBindings, ListenerPaused, ListenerState, SettingsPath,
    bindings_to_registered,
};
```

変更後:
```rust
use coatype_lib::commands::{
    ActiveShortcut, DictPath, ListenerBindings, ListenerPaused, ListenerState, SettingsPath,
    ShowOverlay, bindings_to_registered,
};
```

- [ ] **Step 2: `ShowOverlay` の AtomicBool を初期化して `manage` に登録する**

`src-tauri/src/main.rs` の `setup` クロージャ内、`app.manage(ListenerPaused(paused_arc.clone()));` の直後に追加：

```rust
            let show_overlay_arc = Arc::new(AtomicBool::new(settings.show_overlay));
            app.manage(ShowOverlay(show_overlay_arc.clone()));
```

`settings` 変数はこの時点で既にロード済みなのでそのまま参照できる。

- [ ] **Step 3: `show_overlay_panel` の呼び出しを `ShowOverlay` でガードする**

`src-tauri/src/main.rs` のショートカットループ内の `show_overlay_panel(&handle)` の呼び出し箇所を2か所変更する。

変更前（録音開始時）:
```rust
                                    Ok(()) => {
                                        is_recording = true;
                                        let _ = handle.emit("recording-state", "started");
                                        show_overlay_panel(&handle);
                                    }
```

変更後:
```rust
                                    Ok(()) => {
                                        is_recording = true;
                                        let _ = handle.emit("recording-state", "started");
                                        if show_overlay_arc.load(Ordering::Relaxed) {
                                            show_overlay_panel(&handle);
                                        }
                                    }
```

変更前（`spawn_stop_and_process` 呼び出し直後）:
```rust
            tauri::async_runtime::spawn(async move {
                // HandsFree バインドのトグル状態を main ループで管理
                let mut is_recording = false;
```

`show_overlay_arc` をクロージャにムーブするため、spawn の前に clone する：

```rust
            let show_overlay_for_task = show_overlay_arc.clone();
            tauri::async_runtime::spawn(async move {
                let mut is_recording = false;
```

そして `spawn_stop_and_process` の呼び出しを2か所変更する。

変更前:
```rust
                            spawn_stop_and_process(
                                &pipeline_clone,
                                &handle,
                                prev_pid.lock().unwrap().take(),
                            );
```

変更後（両方の呼び出し箇所）:
```rust
                            spawn_stop_and_process(
                                &pipeline_clone,
                                &handle,
                                prev_pid.lock().unwrap().take(),
                                show_overlay_for_task.load(Ordering::Relaxed),
                            );
```

- [ ] **Step 4: `spawn_stop_and_process` に `show_overlay` 引数を追加する**

`src-tauri/src/main.rs` の `spawn_stop_and_process` 関数シグネチャと中身を変更する。

変更前:
```rust
fn spawn_stop_and_process(
    pipeline: &Arc<Pipeline>,
    handle: &tauri::AppHandle,
    pid: Option<i32>,
) {
    tracing::info!("pipeline: processing start");
    let _ = handle.emit("recording-state", "processing");
    show_overlay_panel(handle);
```

変更後:
```rust
fn spawn_stop_and_process(
    pipeline: &Arc<Pipeline>,
    handle: &tauri::AppHandle,
    pid: Option<i32>,
    show_overlay: bool,
) {
    tracing::info!("pipeline: processing start");
    let _ = handle.emit("recording-state", "processing");
    if show_overlay {
        show_overlay_panel(handle);
    }
```

- [ ] **Step 5: `show_overlay_panel` で位置を計算して設定する**

`src-tauri/src/main.rs` の `show_overlay_panel` 関数全体を以下に置き換える：

```rust
fn show_overlay_panel(handle: &tauri::AppHandle) {
    let h = handle.clone();
    let _ = handle.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            // 画面下部中央に位置を設定 (物理ピクセル)
            if let Ok(Some(monitor)) = w.current_monitor() {
                let scale = monitor.scale_factor();
                let screen_size = monitor.size();
                let screen_pos = monitor.position();
                let overlay_w = (220.0 * scale) as i32;
                let overlay_h = (60.0 * scale) as i32;
                let margin = (40.0 * scale) as i32;
                let x = screen_pos.x + (screen_size.width as i32 - overlay_w) / 2;
                let y = screen_pos.y + screen_size.height as i32 - overlay_h - margin;
                let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
            }
            #[cfg(target_os = "macos")]
            if let Ok(ns_win) = w.ns_window() {
                coatype_lib::focus::show_panel(ns_win);
                return;
            }
            let _ = w.show();
        }
    });
}
```

- [ ] **Step 6: `Ordering` を import に追加する**

`src-tauri/src/main.rs` の既存の行を変更する：

変更前:
```rust
use std::sync::atomic::AtomicBool;
```

変更後:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
```

- [ ] **Step 7: ビルドが通ることを確認する**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "^error" | head -20
```

期待出力: (空 — エラーなし)

- [ ] **Step 8: テスト全体が通ることを確認する**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

期待出力: `test result: ok. N passed`

- [ ] **Step 9: コミット**

```bash
git add src-tauri/src/main.rs
git commit -m "feat(main): ShowOverlay 管理と show_overlay_panel の位置計算・フラグチェックを追加"
```

---

### Task 5: SettingsPage.tsx にオーバーレイ表示トグルを追加

**Files:**
- Modify: `src/SettingsPage.tsx`

- [ ] **Step 1: `GeneralPane` の「オプション」セクションにチェックボックスを追加する**

`src/SettingsPage.tsx` の `GeneralPane` 内、LLM辞書補正の `<div>` ブロックの直後（`<div className="action-bar">` の直前）に追加：

変更前:
```tsx
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
```

変更後:
```tsx
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

      <div>
        <div className="checkbox-row">
          <input
            type="checkbox"
            id="show_overlay"
            checked={settings.show_overlay}
            onChange={(e) => set({ show_overlay: e.target.checked })}
          />
          <label className="checkbox-label" htmlFor="show_overlay">録音中オーバーレイを表示する</label>
        </div>
        <div className="checkbox-desc">録音・処理中にインジケーターを画面下部に表示します</div>
      </div>

      <div className="action-bar">
```

- [ ] **Step 2: TypeScript の型チェックを通す**

```bash
cd /Users/a14161/cotapon/coatype && npx tsc --noEmit 2>&1 | head -20
```

期待出力: (空 — エラーなし)

- [ ] **Step 3: コミット**

```bash
git add src/SettingsPage.tsx
git commit -m "feat(ui): GeneralPane にオーバーレイ表示トグルを追加"
```

---

## 動作確認

- [ ] `npm run tauri dev` でアプリ起動
- [ ] 設定画面 → General → オプション に「録音中オーバーレイを表示する」が表示される
- [ ] ショートカットキーを押したとき、オーバーレイが画面下部中央に表示される
- [ ] 設定でトグルをOFF → 保存 → ショートカットを押してもオーバーレイが出ない
- [ ] トグルをONに戻す → 保存 → ショートカットを押すとオーバーレイが再表示される
