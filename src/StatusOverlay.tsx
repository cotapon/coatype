import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Spinner } from "@heroui/react";
import "./overlay.css";

type RecordingState = "started" | "processing" | "idle";

const WAVE_WIDTH = 132;
// processing 状態の Spinner(sm) = size-4 = 16px に高さを合わせ、状態遷移時のピルの高さ変化をなくす。
const WAVE_HEIGHT = 16;

// アダプティブノイズゲートの絶対下限 (静音環境)。
const NOISE_FLOOR_MIN = 0.006;
// この RMS で振幅が最大になる。小声 (≈0.02) でもフルスケール近くに達するよう設定。
const LEVEL_CEIL = 0.025;

const BAR_COUNT = 24;

/** 録音中の音量 (audio-level) に応じて高さが変わる縦棒バー型オーディオビジュアライザー。 */
function Waveform({ active }: { active: boolean }) {
  const pathRef = useRef<SVGPathElement>(null);
  const levelRef = useRef(0);
  const smoothRef = useRef(0);
  const barLevelsRef = useRef<Float32Array>(new Float32Array(BAR_COUNT));
  const phaseRef = useRef(0);
  // 環境音レベルを動的に追跡するアダプティブノイズフロア。
  const envFloorRef = useRef(NOISE_FLOOR_MIN);

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
      barLevelsRef.current.fill(0);
      envFloorRef.current = NOISE_FLOOR_MIN;
      return;
    }
    let raf = 0;
    const mid = WAVE_HEIGHT / 2;
    const maxAmp = mid - 1.5;
    const render = () => {
      const level = levelRef.current;
      // 声とみなせない範囲（現フロアの2倍以下）でのみ環境音フロアをゆっくり追跡する。
      if (level < envFloorRef.current * 2.0) {
        envFloorRef.current += (level - envFloorRef.current) * 0.01;
        envFloorRef.current = Math.max(NOISE_FLOOR_MIN, envFloorRef.current);
      }
      // 1.2倍マージン: 環境音は多少許容しつつ小声は通す。
      const dynamicFloor = envFloorRef.current * 1.2;
      const norm = (level - dynamicFloor) / (LEVEL_CEIL - dynamicFloor);
      // pow(0.4): 小音量でもフルスケールに早く到達する非線形マッピング。
      const target = Math.pow(Math.max(0, Math.min(1, norm)), 0.4);
      smoothRef.current += (target - smoothRef.current) * 0.2;
      phaseRef.current += 0.25;

      const barW = WAVE_WIDTH / BAR_COUNT;
      let d = "";
      for (let i = 0; i < BAR_COUNT; i++) {
        // 各バーに位相差を持たせて隣同士で高さが波打つようにする。
        const wave =
          Math.sin(i * 0.55 + phaseRef.current) * 0.65 +
          Math.sin(i * 0.2 - phaseRef.current * 0.8) * 0.35;
        // wave は -1〜1 なので abs で常に正の高さにする。
        // 下限0.4を設けて、波の谷でもバーが極端に小さくならないようにする。
        const barTarget = smoothRef.current * Math.max(0.4, Math.abs(wave));
        barLevelsRef.current[i] += (barTarget - barLevelsRef.current[i]) * 0.3;
        const h = Math.max(0.5, maxAmp * barLevelsRef.current[i]);
        const cx = (i + 0.5) * barW;
        d += `M ${cx.toFixed(1)} ${(mid - h).toFixed(1)} L ${cx.toFixed(1)} ${(mid + h).toFixed(1)} `;
      }
      pathRef.current?.setAttribute("d", d);
      const opacity = 0.3 + smoothRef.current * 0.7;
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
        strokeWidth={2.5}
        strokeLinecap="round"
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
