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
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label === "overlay" ? <StatusOverlay /> : <SettingsPage />}
  </React.StrictMode>,
);
