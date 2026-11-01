// Pannello flottante non modale: trascinabile dalla barra del titolo,
// ridimensionabile dall'angolo, sempre dentro la finestra, Esc per chiudere
// (il focus torna al comando che lo ha aperto).
import { useEffect, useId, useRef, type ReactNode } from "react";
import type React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { fit, MIN_H, MIN_W, openers, useFloating } from "../../store/floating";

type Drag = {
  mode: "move" | "resize";
  px: number;
  py: number;
  x: number;
  y: number;
  w: number;
  h: number;
};

export function FloatingPanel({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const titleId = useId();
  const panel = useFloating((s) => s.panels[id]);
  const z = useFloating((s) => s.order.indexOf(id));
  const { setRect, front, close } = useFloating.getState();
  const drag = useRef<Drag | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  // Riconduce il pannello dentro la finestra quando questa si rimpicciolisce.
  useEffect(() => {
    const onResize = () => {
      const p = useFloating.getState().panels[id];
      if (p?.open) setRect(id, fit(p));
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [id, setRect]);

  const isOpen = panel?.open ?? false;
  // All'apertura il focus va al pannello, così Esc lo chiude subito.
  useEffect(() => {
    if (isOpen) ref.current?.focus();
  }, [isOpen]);

  if (!panel?.open) return null;

  const doClose = () => {
    close(id);
    const opener = openers.get(id);
    openers.delete(id);
    opener?.focus();
  };
  const start = (mode: Drag["mode"]) => (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    front(id);
    drag.current = { mode, px: e.clientX, py: e.clientY, ...panel };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  };
  const onMove = (e: React.PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    const dx = e.clientX - d.px;
    const dy = e.clientY - d.py;
    if (d.mode === "move") setRect(id, fit({ x: d.x + dx, y: d.y + dy, w: d.w, h: d.h }));
    else
      setRect(
        id,
        fit({
          x: d.x,
          y: d.y,
          w: Math.max(MIN_W, Math.min(d.w + dx, window.innerWidth - d.x)),
          h: Math.max(MIN_H, Math.min(d.h + dy, window.innerHeight - d.y)),
        }),
      );
  };
  const onUp = (e: React.PointerEvent) => {
    drag.current = null;
    (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
  };

  return createPortal(
    <div
      ref={ref}
      role="dialog"
      aria-modal="false"
      aria-labelledby={titleId}
      data-testid={`floating-${id}`}
      tabIndex={-1}
      onPointerDownCapture={() => front(id)}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          doClose();
        }
      }}
      style={{
        position: "fixed",
        left: panel.x,
        top: panel.y,
        width: panel.w,
        height: panel.h,
        zIndex: 40 + Math.max(0, z),
        display: "flex",
        flexDirection: "column",
        background: "var(--pc-raised)",
        border: "1px solid var(--pc-border)",
        borderRadius: 8,
        boxShadow: "0 8px 28px rgba(0,0,0,0.25)",
        overflow: "hidden",
      }}
    >
      <div
        data-testid={`floating-${id}-title`}
        onPointerDown={start("move")}
        onPointerMove={onMove}
        onPointerUp={onUp}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 8px 6px 12px",
          cursor: "move",
          borderBottom: "1px solid var(--pc-border)",
          userSelect: "none",
          touchAction: "none",
        }}
      >
        <strong id={titleId} style={{ flex: 1, fontSize: 14 }}>
          {title}
        </strong>
        <button
          type="button"
          className="btn btn-ghost"
          style={{ padding: "0 6px" }}
          aria-label={t("library.facets.close")}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={doClose}
        >
          ✕
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        {children}
      </div>
      <div
        data-testid={`floating-${id}-resize`}
        aria-hidden="true"
        onPointerDown={start("resize")}
        onPointerMove={onMove}
        onPointerUp={onUp}
        style={{
          position: "absolute",
          right: 0,
          bottom: 0,
          width: 14,
          height: 14,
          cursor: "nwse-resize",
          touchAction: "none",
        }}
      />
    </div>,
    document.body,
  );
}
