# オーバーレイ表示トグル & 位置変更 設計書

**日付:** 2026-06-09

---

## 概要

録音中に画面に表示されるオーバーレイについて、2点の改善を行う。

1. **表示位置の変更** — 現在のOS任せの位置から、画面下部中央（下端から40px）に固定する
2. **設定画面への表示トグル追加** — ユーザーがオーバーレイの表示・非表示を設定できるようにする

---

## アーキテクチャ

### 1. 表示位置の変更

**対象ファイル:** `src-tauri/src/main.rs`（`show_overlay_panel` 関数）

`show_overlay_panel` 内でウィンドウを表示する前に、`window.set_position()` で位置を計算・設定する。

位置計算ロジック:
- スクリーン情報を `window.current_monitor()` で取得
- `x = (screen_width - 220) / 2`
- `y = screen_height - 60 - 40`（下端から40pxのマージン）

macOS では既存の `show_panel` (orderFront) を維持したまま、その前に `set_position` を呼ぶ。

**依存:** `tauri::PhysicalPosition`, `tauri::PhysicalSize`

---

### 2. Settings 構造体への `show_overlay` フラグ追加

**対象ファイル:** `src-tauri/src/config/settings.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    // ... 既存フィールド ...
    #[serde(default = "default_true")]
    pub show_overlay: bool,
}

fn default_true() -> bool { true }
```

`serde(default = "default_true")` により、既存の settings.json（フィールドなし）を読み込んだ場合も `true` にフォールバックする。

---

### 3. main.rs での `show_overlay` 参照

既存の `ListenerPaused(Arc<AtomicBool>)` パターンと統一し、`Arc<AtomicBool>` で `show_overlay` フラグを manage に登録する。

**変更点:**
- `ListenerPaused` と同様に `ShowOverlay(Arc<AtomicBool>)` 型を追加
- 起動時に `settings.show_overlay` の値で初期化
- `save_settings` コマンドで設定保存時に AtomicBool も更新
- `show_overlay_panel` 呼び出し前に AtomicBool を確認してガード

---

### 4. SettingsPage.tsx への UI 追加

**対象ファイル:** `src/SettingsPage.tsx`（`GeneralPane` コンポーネントの「オプション」セクション）

既存の `translate_mode` / `llm_correct` チェックボックスと同じパターンで追加：

```tsx
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
```

**対象ファイル:** `src/types.ts` — `Settings` 型に `show_overlay: boolean` を追加

---

## データフロー

```
[ショートカット押下]
    → pipeline.start()
    → show_overlay フラグを確認
        → true: show_overlay_panel() → 位置計算 → orderFront
        → false: 何もしない
```

---

## 変更ファイル一覧

| ファイル | 変更内容 |
|---|---|
| `src-tauri/src/config/settings.rs` | `show_overlay: bool` フィールド追加、`default_true` 関数追加 |
| `src-tauri/src/main.rs` | `ShowOverlay(Arc<AtomicBool>)` を manage、`show_overlay` チェック追加、位置計算ロジック追加 |
| `src-tauri/src/commands.rs` | `ShowOverlay` state を受け取り、`save_settings` で AtomicBool を更新 |
| `src/types.ts` | `Settings.show_overlay` 追加 |
| `src/SettingsPage.tsx` | `GeneralPane` にチェックボックス行追加 |

---

## 考慮事項

- **既存 settings.json との互換性:** `serde(default)` により `show_overlay` フィールドがない既存ファイルは `true`（表示あり）として読まれ、挙動が変わらない
- **マルチモニター:** `current_monitor()` はオーバーレイウィンドウが属するモニターを返す。初回表示時はメインモニター扱いになる
- **スクリーン座標:** `PhysicalPosition` を使うため DPI スケールに対応している
