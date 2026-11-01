// Menu contestuale della griglia: agisce sulla selezione (o sulla sola
// miniatura cliccata). Navigabile con le frecce, Invio ed Esc.
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type OpenTarget,
  type PlaceInput,
  type PlaceSuggestion,
} from "../../lib/ipc";
import { PlaceField } from "./PlaceEditor";

const item: CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  border: 0,
  background: "transparent",
  color: "var(--pc-text)",
  padding: "5px 10px",
  fontSize: 13,
  cursor: "pointer",
};

export function GridContextMenu({
  x,
  y,
  ids,
  onOpen,
  onPlace,
  onClose,
}: {
  x: number;
  y: number;
  ids: number[];
  onOpen: (ids: number[], target: OpenTarget) => void;
  onPlace: (ids: number[], place: PlaceInput | null) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const [sub, setSub] = useState(false);
  const [placing, setPlacing] = useState(false);
  const [place, setPlace] = useState("");
  const [picked, setPicked] = useState<PlaceSuggestion | null>(null);
  const config = useQuery({
    queryKey: ["config"],
    queryFn: commands.getConfig,
    enabled: isTauri(),
  });
  const apps = config.data?.external_apps ?? [];

  useEffect(() => {
    ref.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    const away = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [onClose]);

  const act = (fn: () => void) => () => {
    fn();
    onClose();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (placing) return;
    const items = Array.from(ref.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []);
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = items.length;
      items[(i + (e.key === "ArrowDown" ? 1 : n - 1)) % n]?.focus();
    } else if (e.key === "ArrowRight" && document.activeElement?.hasAttribute("aria-haspopup")) {
      e.preventDefault();
      setSub(true);
    } else if (e.key === "ArrowLeft" && sub) {
      e.preventDefault();
      setSub(false);
    }
  };
  useEffect(() => {
    if (sub) ref.current?.querySelector<HTMLElement>('[data-sub] [role="menuitem"]')?.focus();
  }, [sub]);

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={t("externalApps.menu")}
      onKeyDown={onKeyDown}
      onContextMenu={(e) => e.preventDefault()}
      style={{
        position: "fixed",
        left: x,
        top: y,
        zIndex: 20,
        minWidth: 240,
        padding: "4px 0",
        background: "var(--pc-raised)",
        border: "1px solid var(--pc-border)",
        borderRadius: 6,
        boxShadow: "0 6px 20px rgba(0,0,0,0.35)",
      }}
    >
      <div className="muted" style={{ padding: "4px 10px", fontSize: 12 }}>
        {t("externalApps.actsOn", { count: ids.length })}
      </div>
      <button
        type="button"
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={sub}
        style={item}
        onClick={() => setSub(!sub)}
      >
        {t("externalApps.openWith")} ▸
      </button>
      {sub && (
        <div
          role="menu"
          data-sub
          aria-label={t("externalApps.openWith")}
          style={{ paddingLeft: 12 }}
        >
          {apps.map((a) => (
            <button
              key={a.id}
              type="button"
              role="menuitem"
              style={item}
              onClick={act(() => onOpen(ids, { kind: "app", id: a.id ?? "" }))}
            >
              {a.name}
            </button>
          ))}
          <button
            type="button"
            role="menuitem"
            style={item}
            onClick={act(() => onOpen(ids, { kind: "system" }))}
          >
            {t("externalApps.system")}
          </button>
        </div>
      )}
      <button
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => onOpen(ids, { kind: "system" }))}
      >
        {t("externalApps.openDefault")}
      </button>
      <button
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => onOpen(ids, { kind: "reveal" }))}
      >
        {t("externalApps.reveal")}
      </button>
      <hr style={{ border: 0, borderTop: "1px solid var(--pc-border)", margin: "4px 0" }} />
      {placing ? (
        <div style={{ padding: "4px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: 12 }}>{t("place.dialogCount", { count: ids.length })}</span>
          <div style={{ display: "flex", gap: 6 }}>
            <PlaceField
              value={place}
              autoFocus
              onChange={setPlace}
              onPick={(s) => {
                setPlace(s.name);
                setPicked(s);
              }}
            />
            <button
              type="button"
              className="btn"
              disabled={!place.trim()}
              onClick={() => {
                const name = place.trim();
                onPlace(
                  ids,
                  picked && picked.name === name
                    ? { name, country: picked.country, lat: picked.lat, lon: picked.lon }
                    : { name },
                );
                onClose();
              }}
            >
              {t("place.set")}
            </button>
          </div>
        </div>
      ) : (
        <button type="button" role="menuitem" style={item} onClick={() => setPlacing(true)}>
          {t("externalApps.setPlace")}
        </button>
      )}
      <button type="button" role="menuitem" style={item} onClick={act(() => onPlace(ids, null))}>
        {t("externalApps.resetPlace")}
      </button>
      <button
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => void commands.reanalyze(ids))}
      >
        {t("externalApps.reanalyze")}
      </button>
    </div>
  );
}
