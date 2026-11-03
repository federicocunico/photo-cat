// Menu contestuale della griglia: agisce sulla selezione (o sulla sola
// miniatura cliccata). Navigabile con le frecce, Invio ed Esc.
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type OpenTarget,
  type PlaceInput,
  type PlaceSuggestion,
  type Target,
} from "../../lib/ipc";
import { PlaceField } from "./PlaceEditor";

import { useLibrary } from "../../store/library";
import { useCatalogAction } from "./useCatalogAction";

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
  target,
  count = ids.length,
  onOpen,
  onPlace,
  onClose,
}: {
  x: number;
  y: number;
  /** Elementi caricati su cui agisce "Apri con". */
  ids: number[];
  /** Bersaglio delle azioni bulk (anche una selezione "tutti tranne…"). */
  target: Target;
  /** Elementi del bersaglio. */
  count?: number;
  onOpen: (ids: number[], target: OpenTarget) => void;
  onPlace: (target: Target, place: PlaceInput | null) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const catalogAction = useCatalogAction();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
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
      <BusyButton
        type="button"
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={sub}
        style={item}
        onClick={() => setSub(!sub)}
      >
        {t("externalApps.openWith")} ▸
      </BusyButton>
      {sub && (
        <div
          role="menu"
          data-sub
          aria-label={t("externalApps.openWith")}
          style={{ paddingLeft: 12 }}
        >
          {apps.map((a) => (
            <BusyButton
              key={a.id}
              type="button"
              role="menuitem"
              style={item}
              onClick={act(() => onOpen(ids, { kind: "app", id: a.id ?? "" }))}
            >
              {a.name}
            </BusyButton>
          ))}
          <BusyButton
            type="button"
            role="menuitem"
            style={item}
            onClick={act(() => onOpen(ids, { kind: "system" }))}
          >
            {t("externalApps.system")}
          </BusyButton>
        </div>
      )}
      <BusyButton
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => onOpen(ids, { kind: "system" }))}
      >
        {t("externalApps.openDefault")}
      </BusyButton>
      <BusyButton
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => onOpen(ids, { kind: "reveal" }))}
      >
        {t("externalApps.reveal")}
      </BusyButton>
      {(filter.catalogId != null || filter.excludedFrom != null) && (
        <BusyButton
          type="button"
          role="menuitem"
          style={item}
          disabled={catalogAction.busy}
          onClick={act(() => {
            const id = filter.excludedFrom ?? filter.catalogId!;
            void catalogAction.act(
              ids,
              id,
              catalogs.data?.find((c) => c.catalog.id === id)?.catalog.name ?? "",
              filter.excludedFrom != null,
            );
          })}
        >
          {t(filter.excludedFrom != null ? "exclusions.restore" : "exclusions.exclude")}
        </BusyButton>
      )}
      <hr style={{ border: 0, borderTop: "1px solid var(--pc-border)", margin: "4px 0" }} />
      {placing ? (
        <div style={{ padding: "4px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: 12 }}>{t("place.dialogCount", { count })}</span>
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
            <BusyButton
              type="button"
              className="btn"
              disabled={!place.trim()}
              onClick={() => {
                const name = place.trim();
                onPlace(
                  target,
                  picked && picked.name === name
                    ? { name, country: picked.country, lat: picked.lat, lon: picked.lon }
                    : { name },
                );
                onClose();
              }}
            >
              {t("place.set")}
            </BusyButton>
          </div>
        </div>
      ) : (
        <BusyButton type="button" role="menuitem" style={item} onClick={() => setPlacing(true)}>
          {t("externalApps.setPlace")}
        </BusyButton>
      )}
      <BusyButton
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => onPlace(target, null))}
      >
        {t("externalApps.resetPlace")}
      </BusyButton>
      <BusyButton
        type="button"
        role="menuitem"
        style={item}
        onClick={act(() => void commands.reanalyze(target))}
      >
        {t("externalApps.reanalyze")}
      </BusyButton>
    </div>
  );
}
