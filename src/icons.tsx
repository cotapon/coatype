import type { ReactNode } from "react";

// --------------- アイコン (軽量インライン SVG) ---------------

function Svg({ className = "size-4", children }: { className?: string; children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export const IconHome = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M4 11.5 12 4l8 7.5" />
    <path d="M6 10v9.5h12V10" />
  </Svg>
);
export const IconCube = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" />
    <path d="M4 7.5 12 12l8-4.5" />
    <path d="M12 12v9" />
  </Svg>
);
export const IconKey = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="8.5" cy="8.5" r="4" />
    <line x1="11.4" y1="11.4" x2="20" y2="20" />
    <line x1="17.5" y1="17.5" x2="19.5" y2="15.5" />
    <line x1="15" y1="15" x2="17" y2="13" />
  </Svg>
);
export const IconBook = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M5 4.5h9a2 2 0 0 1 2 2V20a2 2 0 0 0-2-2H5z" />
    <path d="M19 6.5V18a2 2 0 0 0-2 2" />
  </Svg>
);
export const IconClock = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="12" cy="12" r="8.2" />
    <path d="M12 7.5v5l3.2 2" />
  </Svg>
);
export const IconMic = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="9" y="3" width="6" height="11" rx="3" />
    <path d="M6 11a6 6 0 0 0 12 0" />
    <line x1="12" y1="17" x2="12" y2="21" />
    <line x1="9" y1="21" x2="15" y2="21" />
  </Svg>
);
export const IconGlobe = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <circle cx="12" cy="12" r="8.5" />
    <path d="M3.5 12h17" />
    <path d="M12 3.5c2.5 2.3 2.5 14.7 0 17M12 3.5c-2.5 2.3-2.5 14.7 0 17" />
  </Svg>
);
export const IconKeyboard = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="3" y="6" width="18" height="12" rx="2" />
    <line x1="7" y1="10" x2="7" y2="10" />
    <line x1="11" y1="10" x2="11" y2="10" />
    <line x1="15" y1="10" x2="15" y2="10" />
    <line x1="8" y1="14" x2="16" y2="14" />
  </Svg>
);
export const IconHand = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M9 11V5.5a1.5 1.5 0 0 1 3 0V11" />
    <path d="M12 10.5v-1a1.5 1.5 0 0 1 3 0V11" />
    <path d="M15 10.5a1.5 1.5 0 0 1 3 0V15a5 5 0 0 1-5 5h-1.5a4 4 0 0 1-3-1.4L5 15.5a1.6 1.6 0 0 1 2.3-2.2L9 15V8a1.5 1.5 0 0 1 3 0" />
  </Svg>
);
export const IconX = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="6" y1="6" x2="18" y2="18" />
    <line x1="18" y1="6" x2="6" y2="18" />
  </Svg>
);
export const IconWaveform = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="5" y1="10" x2="5" y2="14" />
    <line x1="9" y1="7" x2="9" y2="17" />
    <line x1="12" y1="4" x2="12" y2="20" />
    <line x1="15" y1="7" x2="15" y2="17" />
    <line x1="19" y1="10" x2="19" y2="14" />
  </Svg>
);
export const IconPencil = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M4 20h4L19 9l-4-4L4 16v4z" />
    <path d="M14 6l4 4" />
  </Svg>
);
export const IconTrash = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M5 7h14" />
    <path d="M9.5 7V5h5v2" />
    <path d="M7 7l.9 12h8.2L17 7" />
    <line x1="10.5" y1="10.5" x2="10.5" y2="15.5" />
    <line x1="13.5" y1="10.5" x2="13.5" y2="15.5" />
  </Svg>
);
export const IconPlus = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  </Svg>
);
export const IconWarning = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M12 3.5l8.5 15h-17z" />
    <line x1="12" y1="9.5" x2="12" y2="13.5" />
    <circle cx="12" cy="16.5" r="0.8" fill="currentColor" stroke="none" />
  </Svg>
);
export const IconCopy = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </Svg>
);
export const IconUpload = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="17 8 12 3 7 8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </Svg>
);
export const IconRefresh = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M4 12a8 8 0 0 1 14.5-4.5" />
    <path d="M18.5 4v4h-4" />
    <path d="M20 12a8 8 0 0 1-14.5 4.5" />
    <path d="M5.5 20v-4h4" />
  </Svg>
);
export const IconDownload = ({ className }: { className?: string }) => (
  <Svg className={className}>
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="7 10 12 15 17 10" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </Svg>
);
