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
# Tauri 開発サーバー起動 (Rust + Vite のホットリロード)
npm run tauri dev
```

初回はRustのコンパイルに5〜10分かかります。

### テスト

```bash
# プロジェクトルートから実行
npm test
```

テストはすべて [mockito](https://github.com/lipanski/mockito) を使った HTTP モックテストです。実際の API は叩きません。

### デバッグ

#### Rust ログ出力

```bash
RUST_LOG=debug npm run tauri dev
```

`tracing` クレートのログがターミナルに出力されます。キーイベント受信・録音開始/終了・Whisper レスポンスなどが確認できます。

特定クレートのみに絞りたい場合:

```bash
RUST_LOG=coatype=debug,warn npm run tauri dev
```

#### ショートカットのデバッグ

Settings → **General** ペインに「現在有効: `<キー名>` / 状態: ✓ 有効 | ❌」バッジが表示されます。

- `✓ 有効` — リスナーが起動しキーを監視中
- `❌` — アクセシビリティ権限がないか、リスナーの起動に失敗

ショートカットが反応しない場合は `RUST_LOG=debug` でキーイベントがアプリに届いているか確認してください。

#### APIキー / Keychain のデバッグ

```bash
# Keychain に保存されたキーを確認
security find-generic-password -s "jp.co.cyberagent.coatype" -w
```

`COATYPE_API_KEY` 環境変数が設定されている場合は Keychain より優先されます。

#### フロントエンド (React) のデバッグ

`npm run tauri dev` 実行中に DevTools を開けます:

- macOS: `Cmd + Option + I`
- または Settings 画面上で右クリック → 「検証」

---

## 配布 (社内リリース手順)

### リリースビルド

```bash
./scripts/release.sh
```

Apple Silicon + Intel の universal binary DMG が生成されます。  
出力された DMG を社内ストレージ (Google Drive / Slack 等) にアップロードして配布リンクを共有してください。

### インストール手順 (受け取り手向け)

1. DMG をダウンロードしてダブルクリック → `CoAType.app` を `/Applications` にドラッグ
2. **初回起動**: Gatekeeper の警告が出るため、`/Applications/CoAType.app` を **右クリック → 開く** → ダイアログの「開く」をクリック
   - または: 一度ダブルクリック → 警告 → システム設定 → プライバシーとセキュリティ → 下部の「開く」
3. **アクセシビリティ権限を付与**: システム設定 → プライバシーとセキュリティ → アクセシビリティ → CoAType をオンに
4. API キーを設定: メニューバーアイコン → Settings → **API Key** タブ
5. ショートカットで録音 → 文字起こし → 挿入 を確認

> **注意**: 本アプリは ad-hoc 署名 (Apple Developer ID なし) のため初回起動時に警告が出ます。これは仕様です。  
> 警告を消したい場合: `xattr -dr com.apple.quarantine /Applications/CoAType.app` をターミナルで実行してください。

### 署名と自動アップデート (v2 予定)

v2 以降で Apple Developer ID 証明書の取得・Notarization・Tauri Updater による自動配信を予定しています。

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

## 既知の制約 (macOS 26 / Darwin 25.4+)

### 使用できないショートカットキー

macOS 26 (Sequoia 以降) から CGEventTap の動作が変わり、**Session レベルのタップでしか通常キーイベントを受信できなくなりました**。
その結果、以下の組み合わせは動作しません:

| 組み合わせ | 理由 |
|---|---|
| `Shift + Space` など文字キーを含むコンボ | 日本語 IME が Session レベルより先に消費する |
| HID レベルのタップ (rdev デフォルト) | macOS 26 でキーボードイベントが配信されない |

**動作するキー**: Right Option (`⌥`), Left Option, Right Control, Left Control, F5〜F8

### テキスト挿入の制約

クリップボード経由 (`Cmd+V`) でテキストを挿入するため:
- 元のクリップボード内容が上書きされます (挿入後に復元)
- ペーストを受け付けないアプリ (一部ターミナル等) では動作しません

---

## トラブルシューティング

### ショートカットが反応しない

1. アクセシビリティ権限を確認: [手順](#2-アクセシビリティ-グローバルショートカット用)
2. Settings 画面の「現在有効」バッジが `✓ 有効` になっているか確認
3. macOS 26 の場合、`Shift+Space` などの文字キーコンボは使用できません。**Right Option** を推奨

### APIキーを保存したのに 401 エラーになる

Keychain に保存後、アプリを再起動せずにすぐ使えます (in-memory キャッシュが自動更新されます)。
それでも 401 の場合:
- Settings → API Key でキーを再入力して保存
- `COATYPE_API_KEY` 環境変数が設定されている場合はそちらが優先されます

### マイクが使えない

→ システム設定 → プライバシーとセキュリティ → マイク で CoAType を許可

### テキストが入力されない

→ クリップボード経由 (Cmd+V) を使用しています。入力先のアプリがペーストを受け付けることを確認してください

### API エラー

→ `COATYPE_API_KEY` または Keychain のキーが正しいか確認。VPN/ネットワークがCyberAgent内部APIに到達できるか確認。
