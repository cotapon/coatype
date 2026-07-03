import { useState, useEffect, useRef } from "react";

export type SelectionInfo = { text: string; rect: DOMRect; historyId: number };

export function useTextSelection() {
  const [selection, setSelection] = useState<SelectionInfo | null>(null);
  const selectionRef = useRef<SelectionInfo | null>(null);

  const update = (info: SelectionInfo | null) => {
    selectionRef.current = info;
    setSelection(info);
  };

  useEffect(() => {
    const handleSelectionChange = () => {
      const sel = window.getSelection();
      // 有効な選択(単一アイテム内・非空)のみ更新する。
      // 空/コラプス/アイテム跨ぎでは即 null 化しない ─ mousedown ベースのクリアに委ねる。
      if (!sel || sel.isCollapsed || !sel.toString().trim()) return;
      const anchor = sel.anchorNode;
      const focus = sel.focusNode;
      const anchorEl = (anchor instanceof Element ? anchor : anchor?.parentElement)?.closest(
        "[data-history-id]",
      );
      const focusEl = (focus instanceof Element ? focus : focus?.parentElement)?.closest(
        "[data-history-id]",
      );
      if (!anchorEl || !focusEl || anchorEl !== focusEl) return;
      const historyId = Number(anchorEl.getAttribute("data-history-id"));
      if (!historyId) return;
      const range = sel.getRangeAt(0);
      const rect = range.getBoundingClientRect();
      update({ text: sel.toString().trim(), rect, historyId });
    };

    // capture phase で「機能外のクリック」だけを即クリア。
    // [data-history-id](履歴テキスト)・[data-dict-ui](ボタン/ポップオーバー)内は保持。
    const handleMouseDown = (e: MouseEvent) => {
      const target = e.target as Element | null;
      if (!target?.closest("[data-history-id],[data-dict-ui]")) {
        update(null);
      }
    };

    const handleScroll = () => update(null);

    document.addEventListener("selectionchange", handleSelectionChange);
    document.addEventListener("mousedown", handleMouseDown, { capture: true });
    window.addEventListener("scroll", handleScroll, { capture: true });
    return () => {
      document.removeEventListener("selectionchange", handleSelectionChange);
      document.removeEventListener("mousedown", handleMouseDown, { capture: true });
      window.removeEventListener("scroll", handleScroll, { capture: true });
    };
  }, []);

  const clearSelection = () => update(null);

  return { selection, selectionRef, clearSelection };
}
