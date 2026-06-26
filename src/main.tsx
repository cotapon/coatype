import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SettingsPage } from "./SettingsPage";
import { StatusOverlay } from "./StatusOverlay";
import { UpdateDialog } from "./UpdateDialog";
import "./index.css";

const label = getCurrentWebviewWindow().label;

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
