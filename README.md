# CoAType (コエタイプ)

**Voice Typing for CyberAgent** — Powered by whisper-large-v3

音声を録音してWhisper APIで文字起こしし、アクティブなアプリケーションに直接テキストを入力するmacOSデスクトップアプリです。

---

## 機能

- **Push-to-Talk / Toggle** — ショートカットキー長押しまたはトグルで録音
- **whisper-large-v3** — CyberAgent MLプラットフォームのWhisper APIによる高精度文字起こし
- **英語翻訳モード** — `/v1/audio/translations` エンドポイントで日本語→英語翻訳
- **カスタム辞書** — 完全一致文字列置換（常時）＋オプションLLM補正
- **テキスト挿入** — クリップボード経由（Cmd+V）で日本語・絵文字含む全文字に対応
- **文字起こし履歴** — SQLiteに保存、設定画面で確認・削除
- **macOS Keychain** — APIキーを安全に保管

---

## 必要な権限 (macOS)

### 1. マイク

アプリ起動後、macOS がマイクアクセス許可ダイアログを表示します。許可してください。

手動で確認: **システム設定 → プライバシーとセキュリティ → マイク → CoAType**

### 2. アクセシビリティ (グローバルショートカット用)

rdev によるグローバルキー監視にはアクセシビリティ権限が必要です。

**手順:**

1. **システム設定 → プライバシーとセキュリティ → アクセシビリティ** を開く
2. 左下の `+` ボタンをクリックして `CoAType.app` を追加
3. チェックをオンにする

> 権限なしでも起動しますが、ショートカットキーが反応しません。

---

## セットアップ

### 前提条件

- macOS 13.0 (Ventura) 以降
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (stable)
- [Tauri CLI v2](https://tauri.app/)

```bash
npm install -g @tauri-apps/cli
```

### インストール

```bash
git clone https://github.com/cyberagent/coatype
cd coatype
npm install
```

### APIキーの設定

**方法1: 環境変数 (開発時推奨)**

```bash
export COATYPE_API_KEY=mlp-your-key-here
```

**方法2: 設定画面から保存 (Keychain)**

アプリ起動後、メニューバーアイコン → Settings → **API Key** タブでキーを入力して保存。

---

## 開発

```bash
# Tauri 開発サーバー起動
npm run tauri dev
```

初回はRustのコンパイルに数分かかります。

### テスト

```bash
cd src-tauri
cargo test
```

---

## ビルド (配布用)

```bash
npm run tauri build
```

成果物: `src-tauri/target/release/bundle/macos/CoAType.app`

### 署名と公証 (notarization)

`TAURI_SIGNING_PRIVATE_KEY` と Apple Developer証明書が必要です。詳細は [Tauri v2 Code Signing](https://v2.tauri.app/distribute/sign/) を参照。

---

## 設定

メニューバーアイコン → Settings で設定画面を開きます。

| 設定 | 説明 |
|---|---|
| Language | 文字起こし言語コード (例: `ja`, `en`) |
| Shortcut Key | 録音トリガーキー (デフォルト: Right Option) |
| Trigger Mode | Push-to-Talk または Toggle |
| Translate to English | 文字起こし結果を英語に翻訳 |
| LLM Dictionary Correction | 辞書とLLMを使った文脈補正 (実験的) |
| API Base URL | WhisperエンドポイントのベースURL |

---

## トラブルシューティング

### ショートカットが反応しない

→ アクセシビリティ権限を付与してください ([手順](#2-アクセシビリティ-グローバルショートカット用))

### マイクが使えない

→ システム設定 → プライバシーとセキュリティ → マイク で CoAType を許可

### テキストが入力されない

→ クリップボード経由 (Cmd+V) を使用しています。入力先のアプリがペーストを受け付けることを確認してください

### API エラー

→ `COATYPE_API_KEY` または Keychain のキーが正しいか確認。VPN/ネットワークがCyberAgent内部APIに到達できるか確認。
