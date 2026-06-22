import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Spinner } from "@heroui/react";
import "./overlay.css";

type RecordingState = "started" | "processing" | "idle";

const WAVE_WIDTH = 132;
// processing 状態の Spinner(sm) = size-4 = 16px に高さを合わせ、状態遷移時のピルの高さ変化をなくす。
const WAVE_HEIGHT = 16;

// ノイズゲート: この RMS 以下は環境音とみなし波形を平らにする (実測: 無音 ≈ 0.003)。
const NOISE_FLOOR = 0.01;
// この RMS で振幅が最大になる (実測: 発話 ≈ 0.042)。
const LEVEL_CEIL = 0.045;

/** 録音中の音量 (audio-level) に応じて振幅するオシレーター風の波形。 */
function Waveform({ active }: { active: boolean }) {
  const pathRef = useRef<SVGPathElement>(null);
  const levelRef = useRef(0); // Rust から届く生 RMS (実測: 無音 ≈ 0.003, 発話 ≈ 0.042)
  const smoothRef = useRef(0); // 平滑化した振幅 (0〜1)
  const phaseRef = useRef(0);

  useEffect(() => {
    const promise = listen<number>("audio-level", (ev) => {
      levelRef.current = ev.payload;
    });
    return () => {
      promise.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!active) {
      levelRef.current = 0;
      smoothRef.current = 0;
      return;
    }
    let raf = 0;
    const mid = WAVE_HEIGHT / 2;
    const render = () => {
      // ノイズフロアを引いて発話帯域を 0〜1 に再マッピングする。
      // 環境音 (NOISE_FLOOR 以下) は 0 になり、波形が平らになる。
      const norm = (levelRef.current - NOISE_FLOOR) / (LEVEL_CEIL - NOISE_FLOOR);
      const target = Math.max(0, Math.min(1, norm));
      smoothRef.current += (target - smoothRef.current) * 0.25;
      phaseRef.current += 0.35;
      // 振幅は平滑化したレベルにそのまま比例させる (無音時は直線)。
      const amp = (mid - 1) * smoothRef.current;
      let d = "";
      for (let x = 0; x <= WAVE_WIDTH; x += 2) {
        // 2 つの正弦波を重ねてオシレーターらしい揺らぎを出す。
        const y =
          mid +
          Math.sin(x * 0.22 + phaseRef.current) * amp +
          Math.sin(x * 0.07 - phaseRef.current * 0.6) * amp * 0.25;
        d += (x === 0 ? "M" : "L") + x + " " + y.toFixed(1) + " ";
      }
      pathRef.current?.setAttribute("d", d);
      // 無音時は薄く、発話時にはっきり見えるよう不透明度をレベルに連動させる。
      const opacity = 0.25 + smoothRef.current * 0.75;
      pathRef.current?.setAttribute("stroke-opacity", opacity.toFixed(2));
      raf = requestAnimationFrame(render);
    };
    render();
    return () => cancelAnimationFrame(raf);
  }, [active]);

  return (
    <svg
      width={WAVE_WIDTH}
      height={WAVE_HEIGHT}
      viewBox={`0 0 ${WAVE_WIDTH} ${WAVE_HEIGHT}`}
      className="shrink-0"
      aria-hidden
    >
      <path
        ref={pathRef}
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

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
          <Waveform active />
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
