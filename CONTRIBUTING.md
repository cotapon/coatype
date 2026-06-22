# Contributing to CoAType

CoAType に関心を持っていただきありがとうございます。
このドキュメントは、Issue 報告・Pull Request の出し方をまとめたものです。

> **サポート方針について**
> CoAType は個人がメンテナンスしている小規模プロジェクトです。対応はベストエフォートで行っており、
> すべての機能要望や Issue に対応できるとは限りません。あらかじめご了承ください。

---

## Issue を立てる前に

- 既存の [Issues](https://github.com/cotapon/coatype/issues) に同じ報告がないか検索してください。
- バグ報告には **macOS のバージョン**、**再現手順**、**期待した動作と実際の動作** を含めてください。
- 質問・アイデア相談は Issue ではなく [Discussions](https://github.com/cotapon/coatype/discussions) を使ってください（有効な場合）。

セキュリティに関わる問題は **公開 Issue を立てず**、[SECURITY.md](SECURITY.md) の手順に従って報告してください。

---

## 開発環境のセットアップ

詳細は [README.md](README.md) の「セットアップ」を参照してください。要点のみ:

```bash
git clone https://github.com/cotapon/coatype
cd coatype
npm install
npm run tauri dev   # 開発サーバー (Rust + Vite ホットリロード)
```

初回 `cargo build` は依存クレートが多く 5〜10 分かかります。

### テスト

```bash
npm test            # cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Pull Request の出し方

1. リポジトリを **fork** し、作業用ブランチを切る（例: `fix/overlay-height`, `feat/xxx`）。
2. 変更を加え、**テストが通る**ことを確認する（`npm test`）。
3. コミットメッセージは [Conventional Commits](https://www.conventionalcommits.org/) に従う:
   - `fix(scope): ...` バグ修正
   - `feat(scope): ...` 機能追加
   - `docs(scope): ...` ドキュメント
   - `refactor(scope): ...` / `chore(scope): ...` など
4. `main` ブランチ向けに PR を出す。
5. CI（`build.yml`）が通ることを確認する。fork からの PR は CI 実行にメンテナの承認が必要な場合があります。

### PR のレビューについて

- 小さく焦点の絞られた PR ほどレビューが早く進みます。大きな変更は事前に Issue で相談してください。
- メンテナがマージ前にコードを手直しすることがあります。
- プロジェクトの方針に合わない PR は、理由を添えてクローズすることがあります。時間を使っていただいたことには感謝します。

---

## コーディング規約

- フロントエンド: React + TypeScript（`src/`）
- バックエンド: Rust + Tauri v2（`src-tauri/`）
- 既存コードのスタイル・命名・コメントの粒度に合わせてください。
- macOS 固有のハマりどころは [CLAUDE.md](CLAUDE.md) にまとまっています。

---

## ライセンス

コントリビュートされたコードは、本プロジェクトの [MIT License](LICENSE) の下で公開されることに同意したものとみなされます。
