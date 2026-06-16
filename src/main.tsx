import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SettingsPage } from "./SettingsPage";
import { StatusOverlay } from "./StatusOverlay";
import "./index.css";

const label = getCurrentWebviewWindow().label;

if (label === "overlay") {
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
} else {
  // 設定ウィンドウは OS のライト / ダーク設定に追従する。
  // (オーバーレイは透過維持のため対象外)
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
    {label === "overlay" ? <StatusOverlay /> : <SettingsPage />}
  </React.StrictMode>,
);
