// Layout a tre colonne del modulo: pannello sinistro e ispettore ridimensionabili
// (trascinamento o frecce da tastiera) e richiudibili; lo stato è persistito.

import { useRef, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { useLayout } from "../../store/layout";
import { useCommands } from "../../commands/registry";
import "./shell.css";

/**
 * Ctrl+I mostra o nasconde l'ispettore; Ctrl+Maiusc+B nasconde entrambi i pannelli
 * laterali e li ripristina com'erano. Tab resta allo spostamento del focus.
 */
function usePanelCommands(hasRight: boolean) {
  const { t } = useTranslation();
  useCommands([
    {
      id: "panels.inspector",
      label: t("commands.toggleInspector"),
      group: "library",
      shortcut: { key: "i", mod: true },
      when: () => hasRight,
      run: () => useLayout.getState().toggleRight(),
    },
    {
      id: "panels.sides",
      label: t("commands.toggleSidePanels"),
      group: "library",
      shortcut: { key: "b", mod: true, shift: true },
      run: () => useLayout.getState().toggleSidePanels(),
    },
  ]);
}

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
    <div className="panel-rail" data-side={side}>
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
  usePanelCommands(Boolean(right));
  return (
    <div className="panels">
      {left &&
        (s.leftOpen ? (
          <>
            <aside
              data-testid="left-panel"
              className="panel-aside panel-aside-left"
              style={{ width: s.leftWidth }}
            >
              <button
                type="button"
                className="btn btn-ghost icon-btn panel-collapse panel-collapse-left"
                aria-label={t("shell.collapseLeft")}
                onClick={s.toggleLeft}
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
      <section className="panels-center">{center}</section>
      {right &&
        (s.rightOpen ? (
          <>
            <Resizer
              label={t("shell.resizeRight")}
              onDelta={(dx) => s.setRightWidth(s.rightWidth - dx)}
            />
            <aside
              data-testid="right-panel"
              className="panel-aside panel-aside-right"
              style={{ width: s.rightWidth }}
            >
              <button
                type="button"
                className="btn btn-ghost icon-btn panel-collapse panel-collapse-right"
                aria-label={t("shell.collapseRight")}
                aria-keyshortcuts="Control+I"
                onClick={s.toggleRight}
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
