import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import "./overlay.css";

type RecordingState = "started" | "processing" | "idle";

export function StatusOverlay() {
  const [state, setState] = useState<RecordingState>("idle");

  useEffect(() => {
    const promise = listen<RecordingState>("recording-state", (ev) => {
      setState(ev.payload);
    });
    return () => {
      promise.then((fn) => fn());
    };
  }, []);

  if (state === "idle") return null;

  return (
    <div className="overlay-pill">
      {state === "started" && (
        <>
          <span className="rec-dot" />
          <span>Recording…</span>
        </>
      )}
      {state === "processing" && (
        <>
          <span className="spinner" />
          <span>Processing…</span>
        </>
      )}
    </div>
  );
}
