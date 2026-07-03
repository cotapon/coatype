import { defineConfig } from "eslint/config";
import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default defineConfig(
  { ignores: ["dist/", "node_modules/", "src-tauri/", "public/", ".cargo-home/"] },
  {
    files: ["src/**/*.{ts,tsx}", "vite.config.ts"],
    extends: [js.configs.recommended, tseslint.configs.recommended],
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // マウント時の初期ロード (setState を伴う fetch) が既存 UI の基本パターンのため無効化。
      // データ取得層を整理するタイミングで再有効化を検討する。
      "react-hooks/set-state-in-effect": "off",
    },
  },
);
