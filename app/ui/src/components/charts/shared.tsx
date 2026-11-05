import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import "./charts.css";

/** Larghezza usata quando il contenitore non è misurabile (jsdom, primo render). */
export const FALLBACK_WIDTH = 480;

/** Larghezza del contenitore, aggiornata con ResizeObserver. */
export function useWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const read = () => setWidth(Math.round(el.getBoundingClientRect().width));
    read();
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return { ref, width: width > 40 ? width : FALLBACK_WIDTH };
}

interface TipState {
  x: number;
  y: number;
  content: ReactNode;
}

/** Tooltip leggero: posizione nello spazio del contenitore, mostrato anche al focus. */
export function useTip(width: number) {
  const [tip, setTip] = useState<TipState | null>(null);
  const show = useCallback((x: number, y: number, content: ReactNode) => setTip({ x, y, content }), []);
  const hide = useCallback(() => setTip(null), []);
  const node = tip ? (
    <div
      className="pc-chart-tip"
      role="tooltip"
      style={{ left: Math.min(Math.max(tip.x, 60), Math.max(60, width - 60)), top: Math.max(tip.y - 6, 24) }}
    >
      {tip.content}
    </div>
  ) : null;
  return { show, hide, node };
}

/** Invio o Spazio attivano l'elemento. */
export function activateOnKey(fn: () => void) {
  return (e: KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      fn();
    }
  };
}

export function EmptyState({ text }: { text: ReactNode }) {
  return (
    <div className="pc-chart-empty" role="status">
      {text}
    </div>
  );
}

export function truncate(s: string, max: number) {
  return s.length > max ? `${s.slice(0, Math.max(1, max - 1))}…` : s;
}
