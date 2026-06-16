import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Spinner } from "@heroui/react";
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
    <div className="flex select-none items-center gap-2 rounded-[20px] bg-[rgba(20,20,20,0.82)] px-4 py-2 text-[13px] font-medium tracking-wide text-white backdrop-blur-md">
      {state === "started" && (
        <>
          <span className="rec-dot h-2 w-2 shrink-0 rounded-full bg-[#ff3b30]" />
          <span>Recording…</span>
        </>
      )}
      {state === "processing" && (
        <>
          <Spinner size="sm" color="current" />
          <span>Processing…</span>
        </>
      )}
    </div>
  );
}
