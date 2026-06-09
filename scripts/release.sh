#!/usr/bin/env bash
set -euo pipefail

# Universal binary をビルドして DMG パスを出力する。
# 配布はこのスクリプトの出力 DMG を社内ストレージに手動で上げる。
#
# 使い方:
#   ./scripts/release.sh
#
# 前提:
#   - npm install 済み
#   - rustup インストール済み

cd "$(dirname "$0")/.."

# ターゲット導入をベキ等に
rustup target add x86_64-apple-darwin aarch64-apple-darwin 1>/dev/null

echo "=== CoAType リリースビルド ==="
echo "バージョン: $(jq -r .version src-tauri/tauri.conf.json)"
echo ""

# Universal binary ビルド (Tauri が ad-hoc codesign を自動実行)
npm run tauri build -- --target universal-apple-darwin

# 成果物パスを出力
BUNDLE_DIR="src-tauri/target/universal-apple-darwin/release/bundle"
DMG_PATH=$(ls "$BUNDLE_DIR/dmg/"*.dmg 2>/dev/null | head -1)

if [[ -z "$DMG_PATH" ]]; then
  echo "ERROR: DMG が生成されませんでした" >&2
  exit 1
fi

echo ""
echo "=== Build complete ==="
echo "DMG: $DMG_PATH"
echo "size: $(du -h "$DMG_PATH" | cut -f1)"
echo ""
echo "Next: 上記 DMG を社内ストレージ (Google Drive / Slack 等) にアップロードして配布リンクを共有してください。"
echo "      インストール手順は README.md の「インストール手順」セクションを参照してください。"
