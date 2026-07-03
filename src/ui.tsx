import type { ReactNode } from "react";

// --------------- 共通レイアウト要素 ---------------

/** ペイン見出し (タイトル + サブタイトル) */
export function PaneHeader({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <div className="mb-6">
      <h1 className="text-[26px] font-bold tracking-tight text-foreground">{title}</h1>
      <p className="mt-1 text-sm text-muted">{subtitle}</p>
    </div>
  );
}

/** 太字の中見出しを持つセクション */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-7">
      <h2 className="mb-3 text-[15px] font-bold text-foreground">{title}</h2>
      {children}
    </section>
  );
}

/** 白いカードコンテナ (rounded-xl + 枠線 + 影) */
export function Panel({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={`rounded-xl border border-border bg-surface shadow-[var(--surface-shadow)] ${className}`}
    >
      {children}
    </div>
  );
}

/** 小さな丸アイコンタイル (淡いアクセント背景) */
export function IconTile({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`flex size-9 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent ${className}`}
    >
      {children}
    </div>
  );
}
