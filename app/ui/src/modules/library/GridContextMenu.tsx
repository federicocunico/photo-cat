// Menu contestuale della griglia: agisce sulla selezione (o sulla sola
// miniatura cliccata). Navigabile con le frecce, Invio ed Esc.
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
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
import { Button, Dialog, FormField, SearchPicker } from "../../components/ui";
import { notifyError } from "../../store/toasts";

import { useLibrary } from "../../store/library";
import { openReprocessFor } from "../ai/reprocess/openReprocess";
import { useCatalogAction } from "./useCatalogAction";
import { Icon } from "../../components/icons";

export function GridContextMenu({
  x,
  y,
  ids,
  target,
  count = ids.length,
  onOpen,
  onPlace,
  onAddToCatalog,
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
  /** Aggiunge il bersaglio a un catalogo manuale (avviso e annullamento a cura di chi chiama). */
  onAddToCatalog: (target: Target, count: number, catalog: number, name: string) => void;
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
  const qc = useQueryClient();
  const [sub, setSub] = useState(false);
  const [catSub, setCatSub] = useState(false);
  const [naming, setNaming] = useState(false);
  const [newName, setNewName] = useState("");
  const [nameError, setNameError] = useState("");
  const namingRef = useRef(false);
  namingRef.current = naming;
  const manual = (catalogs.data ?? [])
    .map((c) => c.catalog)
    .filter((c) => c.kind === "manual" && !c.system)
    .sort((a, b) => a.name.localeCompare(b.name));
  const NEW = "new";
  const catalogOptions = [
    ...manual.map((c) => ({ value: String(c.id), label: c.name })),
    { value: NEW, label: t("selection.newCatalogMenu") },
  ];
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
      if (!namingRef.current && ref.current && !ref.current.contains(e.target as Node)) onClose();
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
    if (placing || (e.target as HTMLElement).closest("[data-catalog-sub]")) return;
    const items = Array.from(ref.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []);
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = items.length;
      items[(i + (e.key === "ArrowDown" ? 1 : n - 1)) % n]?.focus();
    } else if (e.key === "ArrowRight" && document.activeElement?.hasAttribute("aria-haspopup")) {
      e.preventDefault();
      if (document.activeElement?.getAttribute("data-catalog-trigger") != null) {
        setCatSub(true);
        setSub(false);
      } else {
        setSub(true);
        setCatSub(false);
      }
    } else if (e.key === "ArrowLeft" && (sub || catSub)) {
      e.preventDefault();
      setSub(false);
      setCatSub(false);
    }
  };
  useEffect(() => {
    if (sub) ref.current?.querySelector<HTMLElement>('[data-sub] [role="menuitem"]')?.focus();
  }, [sub]);
  useEffect(() => {
    if (catSub) ref.current?.querySelector<HTMLElement>("[data-catalog-sub] input")?.focus();
  }, [catSub]);

  const createAndAdd = async () => {
    const name = newName.trim();
    if (!name) return;
    try {
      const id = await commands.saveCatalog(null, name, "manual", null);
      await qc.invalidateQueries({ queryKey: ["catalogs"] });
      onAddToCatalog(target, count, id, name);
      onClose();
    } catch (error) {
      setNameError(String(error));
      notifyError(error);
    }
  };

  return (
    <>
      <div
        ref={ref}
        role="menu"
        aria-label={t("externalApps.menu")}
        onKeyDown={onKeyDown}
        onContextMenu={(e) => e.preventDefault()}
        className="grid-menu"
        style={{ left: x, top: y, display: naming ? "none" : undefined }}
      >
        <div className="muted grid-menu-hint">
          {t("externalApps.actsOn", { count: ids.length })}
        </div>
        <BusyButton
          type="button"
          role="menuitem"
          aria-haspopup="menu"
          aria-expanded={sub}
          className="grid-menu-item"
          onClick={() => {
            setSub(!sub);
            setCatSub(false);
          }}
        >
          {t("externalApps.openWith")} <Icon.ChevronRight size={14} />
        </BusyButton>
        {sub && (
          <div
            role="menu"
            data-sub
            aria-label={t("externalApps.openWith")}
            className="grid-menu-sub"
          >
            {apps.map((a) => (
              <BusyButton
                key={a.id}
                type="button"
                role="menuitem"
                className="grid-menu-item"
                onClick={act(() => onOpen(ids, { kind: "app", id: a.id ?? "" }))}
              >
                {a.name}
              </BusyButton>
            ))}
            <BusyButton
              type="button"
              role="menuitem"
              className="grid-menu-item"
              onClick={act(() => onOpen(ids, { kind: "system" }))}
            >
              {t("externalApps.system")}
            </BusyButton>
          </div>
        )}
        <BusyButton
          type="button"
          role="menuitem"
          aria-haspopup="menu"
          aria-expanded={catSub}
          data-catalog-trigger=""
          className="grid-menu-item"
          onClick={() => {
            setCatSub(!catSub);
            setSub(false);
          }}
        >
          {t("selection.addToCatalog")} <Icon.ChevronRight size={14} />
        </BusyButton>
        {catSub && (
          <div data-catalog-sub className="grid-menu-sub">
            <SearchPicker
              label={t("selection.catalogSearch")}
              placeholder={t("selection.catalogSearch")}
              options={catalogOptions}
              emptyLabel={t("selection.noCatalogs")}
              onPick={(value) => {
                if (value === NEW) {
                  setNewName("");
                  setNameError("");
                  setNaming(true);
                  return;
                }
                const id = Number(value);
                onAddToCatalog(target, count, id, manual.find((c) => c.id === id)?.name ?? "");
                onClose();
              }}
            />
          </div>
        )}
        <BusyButton
          type="button"
          role="menuitem"
          className="grid-menu-item"
          onClick={act(() => onOpen(ids, { kind: "system" }))}
        >
          {t("externalApps.openDefault")}
        </BusyButton>
        <BusyButton
          type="button"
          role="menuitem"
          className="grid-menu-item"
          onClick={act(() => onOpen(ids, { kind: "reveal" }))}
        >
          {t("externalApps.reveal")}
        </BusyButton>
        {(filter.catalogId != null || filter.excludedFrom != null) && (
          <BusyButton
            type="button"
            role="menuitem"
            className="grid-menu-item"
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
        <hr className="grid-menu-sep" />
        {placing ? (
          <div className="grid-menu-place">
            <span className="grid-menu-place-label">{t("place.dialogCount", { count })}</span>
            <div className="grid-menu-place-row">
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
          <BusyButton
            type="button"
            role="menuitem"
            className="grid-menu-item"
            onClick={() => setPlacing(true)}
          >
            {t("externalApps.setPlace")}
          </BusyButton>
        )}
        <BusyButton
          type="button"
          role="menuitem"
          className="grid-menu-item"
          onClick={act(() => onPlace(target, null))}
        >
          {t("externalApps.resetPlace")}
        </BusyButton>
        <BusyButton
          type="button"
          role="menuitem"
          className="grid-menu-item"
          onClick={act(() => openReprocessFor(target, count))}
        >
          {t("externalApps.reanalyze")}
        </BusyButton>
      </div>
      <Dialog
        open={naming}
        onOpenChange={(o) => {
          if (!o) onClose();
        }}
        title={t("selection.newCatalogTitle")}
        width={380}
        footer={
          <>
            <Button onClick={onClose}>{t("common.cancel")}</Button>
            <Button variant="primary" disabled={!newName.trim()} onClick={createAndAdd}>
              {t("selection.newCatalogCreateBtn")}
            </Button>
          </>
        }
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void createAndAdd();
          }}
        >
          <FormField label={t("selection.newCatalogName")} error={nameError || undefined}>
            <input
              className="field"
              autoFocus
              value={newName}
              onChange={(e) => {
                setNewName(e.target.value);
                setNameError("");
              }}
            />
          </FormField>
        </form>
      </Dialog>
    </>
  );
}
