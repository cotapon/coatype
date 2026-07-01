import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SettingsPage } from "./SettingsPage";
import { StatusOverlay } from "./StatusOverlay";
import { UpdateDialog } from "./UpdateDialog";
import { getPlatform } from "./invoke";
import { setPlatform } from "./types";
import "./index.css";

const label = getCurrentWebviewWindow().label;

// 修飾キーの表示ラベル(⌘/⌥/⌃ vs Win/Alt/Ctrl)を実行 OS に合わせて切り替えるため、
// 起動時に1回 OS を取得しておく。取得前は "macos" 表記のまま(既存挙動維持)。
getPlatform()
  .then(setPlatform)
  .catch(() => {
    // 取得に失敗しても表示ラベルが macOS 表記のままになるだけで、機能上の実害はない。
  });

// 透過ウィンドウ(overlay と update-dialog)は html/body を透明に。
// 角丸カードの外側が透過されるようにするため。
if (label === "overlay" || label === "update-dialog") {
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
}

// overlay 以外(update-dialog を含む)は OS のライト/ダーク設定に追従する。
if (label !== "overlay") {
  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const applySystemTheme = () => {
    const root = document.documentElement;
    const isDark = darkQuery.matches;
    root.classList.toggle("dark", isDark);
    root.classList.toggle("light", !isDark);
    root.setAttribute("data-theme", isDark ? "dark" : "light");
  };
  applySystemTheme();
  darkQuery.addEventListener("change", applySystemTheme);
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label === "overlay" ? (
      <StatusOverlay />
    ) : label === "update-dialog" ? (
      <UpdateDialog />
    ) : (
      <SettingsPage />
    )}
  </React.StrictMode>,
);
