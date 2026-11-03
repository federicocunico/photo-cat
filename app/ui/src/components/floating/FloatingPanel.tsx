// Pannello flottante non modale: trascinabile dalla barra del titolo, ridimensionabile
// dall'angolo, sempre dentro la finestra, Esc per chiudere (il focus torna al comando che
// lo ha aperto). Senza mouse: menu delle opzioni del pannello › Sposta / Ridimensiona, poi
// frecce (Maiusc per passi lunghi), Invio o Esc per finire (WCAG 2.5.7).
import "./floating.css";
import { useEffect, useId, useRef, useState, type CSSProperties, type ReactNode } from "react";
import type React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { fit, MIN_H, MIN_W, openers, useFloating, type PanelRect } from "../../store/floating";
import { Icon } from "../icons";
import { IconButton, Menu } from "../ui";

type Drag = PanelRect & { mode: KeyMode; px: number; py: number };

type KeyMode = "move" | "resize";

/** Passo delle frecce in pixel: normale e con Maiusc. */
const KEY_STEP = 10;
const KEY_STEP_LONG = 50;

const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/** Nuovo rettangolo dopo lo spostamento o il ridimensionamento di (dx, dy). */
function moved(mode: KeyMode, r: PanelRect, dx: number, dy: number): PanelRect {
  if (mode === "move") return fit({ x: r.x + dx, y: r.y + dy, w: r.w, h: r.h });
  return fit({
    x: r.x,
    y: r.y,
    w: Math.max(MIN_W, Math.min(r.w + dx, window.innerWidth - r.x)),
    h: Math.max(MIN_H, Math.min(r.h + dy, window.innerHeight - r.y)),
  });
}

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
  const layer = useFloating((s) => s.order.indexOf(id));
  const { setRect, front, close } = useFloating.getState();
  const drag = useRef<Drag | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const [keyMode, setKeyMode] = useState<KeyMode | null>(null);

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
    else setKeyMode(null);
  }, [isOpen]);

  if (!panel?.open) return null;

  const doClose = () => {
    close(id);
    const opener = openers.get(id);
    openers.delete(id);
    opener?.focus();
  };
  const start = (mode: KeyMode) => (e: React.PointerEvent) => {
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
    setRect(id, moved(d.mode, d, e.clientX - d.px, e.clientY - d.py));
  };
  const onUp = (e: React.PointerEvent) => {
    drag.current = null;
    (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
  };

  // I tasti dei menu aperti dal pannello (in un portale) risalgono l'albero di React:
  // il pannello gestisce solo quelli premuti al suo interno.
  const inside = (e: React.KeyboardEvent) => ref.current?.contains(e.target as Node) ?? false;
  // In modalità Sposta/Ridimensiona le frecce arrivano prima al pannello (fase di cattura),
  // così i controlli interni (es. il pulsante del menu) non le usano.
  const onKeyDownCapture = (e: React.KeyboardEvent) => {
    if (!keyMode || !inside(e)) return;
    const dir = ARROWS[e.key];
    if (dir) {
      e.preventDefault();
      e.stopPropagation();
      const step = e.shiftKey ? KEY_STEP_LONG : KEY_STEP;
      setRect(id, moved(keyMode, panel, dir[0] * step, dir[1] * step));
    } else if (e.key === "Escape" || e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      setKeyMode(null);
    }
  };
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!inside(e)) return;
    if (e.key === "Escape") {
      e.stopPropagation();
      doClose();
    }
  };

  const arrows = t("floating.arrows");
  const style = {
    left: panel.x,
    top: panel.y,
    width: panel.w,
    height: panel.h,
    // Ordine di sovrapposizione tra i pannelli, sopra il livello `--pc-z-floating`.
    "--floating-layer": Math.max(0, layer),
  } as CSSProperties;

  return createPortal(
    <div
      ref={ref}
      role="dialog"
      aria-modal="false"
      aria-labelledby={titleId}
      data-testid={`floating-${id}`}
      data-key-mode={keyMode ?? undefined}
      className="floating-panel"
      tabIndex={-1}
      onPointerDownCapture={() => front(id)}
      onFocus={() => front(id)}
      onBlur={(e) => {
        if (!ref.current?.contains(e.relatedTarget as Node | null)) setKeyMode(null);
      }}
      onKeyDownCapture={onKeyDownCapture}
      onKeyDown={onKeyDown}
      style={style}
    >
      <div
        data-testid={`floating-${id}-title`}
        className="floating-title"
        onPointerDown={start("move")}
        onPointerMove={onMove}
        onPointerUp={onUp}
      >
        <strong id={titleId}>{title}</strong>
        <span className="floating-tools" onPointerDown={(e) => e.stopPropagation()}>
          <Menu
            align="end"
            trigger={
              <button
                type="button"
                className="btn btn-ghost icon-btn"
                aria-label={t("floating.options", { name: title })}
              >
                <Icon.More size={16} />
              </button>
            }
            items={[
              {
                label: t("floating.move"),
                shortcut: arrows,
                onSelect: () => {
                  front(id);
                  setKeyMode("move");
                },
              },
              {
                label: t("floating.resize"),
                shortcut: arrows,
                onSelect: () => {
                  front(id);
                  setKeyMode("resize");
                },
              },
              { kind: "separator" },
              { label: t("floating.closeItem"), shortcut: "Esc", onSelect: doClose },
            ]}
          />
          <IconButton
            label={t("floating.close", { name: title })}
            icon={<Icon.Close size={16} />}
            shortcut="Esc"
            onClick={doClose}
          />
        </span>
      </div>
      {keyMode && (
        <p role="status" className="floating-keys">
          {t(keyMode === "move" ? "floating.moving" : "floating.resizing")}
        </p>
      )}
      <div className="floating-body">{children}</div>
      <div
        data-testid={`floating-${id}-resize`}
        aria-hidden="true"
        className="floating-resize"
        onPointerDown={start("resize")}
        onPointerMove={onMove}
        onPointerUp={onUp}
      />
    </div>,
    document.body,
  );
}
