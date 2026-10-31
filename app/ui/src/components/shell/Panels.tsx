// Layout a tre colonne del modulo: pannello sinistro e ispettore ridimensionabili
// (trascinamento o frecce da tastiera) e richiudibili; lo stato è persistito.

import { useRef, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { useLayout } from "../../store/layout";

function Resizer({ label, onDelta }: { label: string; onDelta: (dx: number) => void }) {
  const last = useRef<number | null>(null);
  const down = (e: PointerEvent<HTMLButtonElement>) => {
    last.current = e.clientX;
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const move = (e: PointerEvent<HTMLButtonElement>) => {
    if (last.current === null) return;
    onDelta(e.clientX - last.current);
    last.current = e.clientX;
  };
  const up = () => {
    last.current = null;
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === "ArrowLeft") onDelta(-16);
    if (e.key === "ArrowRight") onDelta(16);
  };
  return (
    <button
      type="button"
      className="resizer"
      aria-label={label}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onKeyDown={key}
    />
  );
}

function CollapsedRail({
  side,
  label,
  onClick,
}: {
  side: "left" | "right";
  label: string;
  onClick: () => void;
}) {
  return (
    <div
      style={{
        flex: "0 0 32px",
        display: "flex",
        justifyContent: "center",
        paddingTop: 8,
        background: "var(--pc-panel-alt)",
        borderLeft: side === "right" ? "1px solid var(--pc-border)" : undefined,
        borderRight: side === "left" ? "1px solid var(--pc-border)" : undefined,
      }}
    >
      <button type="button" className="btn btn-ghost icon-btn" aria-label={label} onClick={onClick}>
        {side === "left" ? <Icon.ChevronRight size={16} /> : <Icon.ChevronLeft size={16} />}
      </button>
    </div>
  );
}

export function Panels({
  left,
  center,
  right,
}: {
  left?: ReactNode;
  center: ReactNode;
  right?: ReactNode;
}) {
  const { t } = useTranslation();
  const s = useLayout();
  return (
    <div style={{ flex: 1, display: "flex", minWidth: 0, minHeight: 0 }}>
      {left &&
        (s.leftOpen ? (
          <>
            <aside
              data-testid="left-panel"
              style={{
                width: s.leftWidth,
                flex: "0 0 auto",
                background: "var(--pc-panel-alt)",
                borderRight: "1px solid var(--pc-border)",
                overflowY: "auto",
                position: "relative",
              }}
            >
              <button
                type="button"
                className="btn btn-ghost icon-btn"
                aria-label={t("shell.collapseLeft")}
                onClick={s.toggleLeft}
                style={{ position: "absolute", top: 8, right: 4, border: 0 }}
              >
                <Icon.ChevronLeft size={16} />
              </button>
              {left}
            </aside>
            <Resizer
              label={t("shell.resizeLeft")}
              onDelta={(dx) => s.setLeftWidth(s.leftWidth + dx)}
            />
          </>
        ) : (
          <CollapsedRail side="left" label={t("shell.expandLeft")} onClick={s.toggleLeft} />
        ))}
      <section
        style={{
          flex: 1,
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {center}
      </section>
      {right &&
        (s.rightOpen ? (
          <>
            <Resizer
              label={t("shell.resizeRight")}
              onDelta={(dx) => s.setRightWidth(s.rightWidth - dx)}
            />
            <aside
              data-testid="right-panel"
              style={{
                width: s.rightWidth,
                flex: "0 0 auto",
                background: "var(--pc-panel)",
                borderLeft: "1px solid var(--pc-border)",
                overflowY: "auto",
                position: "relative",
              }}
            >
              <button
                type="button"
                className="btn btn-ghost icon-btn"
                aria-label={t("shell.collapseRight")}
                onClick={s.toggleRight}
                style={{ position: "absolute", top: 8, right: 8, border: 0 }}
              >
                <Icon.ChevronRight size={16} />
              </button>
              {right}
            </aside>
          </>
        ) : (
          <CollapsedRail side="right" label={t("shell.expandRight")} onClick={s.toggleRight} />
        ))}
    </div>
  );
}
