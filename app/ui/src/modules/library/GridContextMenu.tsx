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
import { ManualDateDialog } from "./ManualDateDialog";
import type { BulkAction, ColorTrait } from "../../bindings";

const COLOR_TRAITS: Array<ColorTrait | null> = ["bw", "color", "faded", "sepia", null];

export function GridContextMenu({
  x,
  y,
  ids,
  target,
  count = ids.length,
  onOpen,
  onPlace,
  onAddToCatalog,
  onBulk,
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
  /** Valutazione, colore e data di scatto sul bersaglio (avviso e annullamento a cura di chi chiama). */
  onBulk?: (target: Target, operation: BulkAction, count: number) => void;
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
  const [extra, setExtra] = useState<"rating" | "color" | null>(null);
  const [dating, setDating] = useState(false);
  const [naming, setNaming] = useState(false);
  const [newName, setNewName] = useState("");
  const [nameError, setNameError] = useState("");
  const namingRef = useRef(false);
  namingRef.current = naming;
  const datingRef = useRef(false);
  datingRef.current = dating;
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
      if (
        !namingRef.current &&
        !datingRef.current &&
        ref.current &&
        !ref.current.contains(e.target as Node)
      )
        onClose();
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
      const extraKind = document.activeElement?.getAttribute("data-extra") as
        "rating" | "color" | null;
      if (extraKind) {
        setExtra(extraKind);
        setSub(false);
        setCatSub(false);
      } else if (document.activeElement?.getAttribute("data-catalog-trigger") != null) {
        setCatSub(true);
        setSub(false);
        setExtra(null);
      } else {
        setSub(true);
        setCatSub(false);
        setExtra(null);
      }
    } else if (e.key === "ArrowLeft" && (sub || catSub || extra)) {
      e.preventDefault();
      setSub(false);
      setCatSub(false);
      setExtra(null);
    }
  };
  useEffect(() => {
    if (sub) ref.current?.querySelector<HTMLElement>('[data-sub] [role="menuitem"]')?.focus();
  }, [sub]);
  useEffect(() => {
    if (extra)
      ref.current?.querySelector<HTMLElement>('[data-extra-sub] [role="menuitem"]')?.focus();
  }, [extra]);
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
        style={{ left: x, top: y, display: naming || dating ? "none" : undefined }}
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
            setExtra(null);
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
            setExtra(null);
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
        {onBulk && (
          <>
            <BusyButton
              type="button"
              role="menuitem"
              aria-haspopup="menu"
              aria-expanded={extra === "rating"}
              data-extra="rating"
              className="grid-menu-item"
              onClick={() => {
                setExtra(extra === "rating" ? null : "rating");
                setSub(false);
                setCatSub(false);
              }}
            >
              {t("selection.rateMenu")} <Icon.ChevronRight size={14} />
            </BusyButton>
            {extra === "rating" && (
              <div
                role="menu"
                data-extra-sub
                aria-label={t("selection.rateMenu")}
                className="grid-menu-sub"
              >
                {[1, 2, 3, 4, 5].map((n) => (
                  <BusyButton
                    key={n}
                    type="button"
                    role="menuitem"
                    className="grid-menu-item"
                    aria-label={t("library.inspector.stars", { count: n })}
                    onClick={act(() => onBulk(target, { action: "set_rating", rating: n }, count))}
                  >
                    {"★".repeat(n)}
                  </BusyButton>
                ))}
                <BusyButton
                  type="button"
                  role="menuitem"
                  className="grid-menu-item"
                  onClick={act(() => onBulk(target, { action: "set_rating", rating: null }, count))}
                >
                  {t("library.inspector.noRating")}
                </BusyButton>
              </div>
            )}
            <BusyButton
              type="button"
              role="menuitem"
              aria-haspopup="menu"
              aria-expanded={extra === "color"}
              data-extra="color"
              className="grid-menu-item"
              onClick={() => {
                setExtra(extra === "color" ? null : "color");
                setSub(false);
                setCatSub(false);
              }}
            >
              {t("selection.colorMenu")} <Icon.ChevronRight size={14} />
            </BusyButton>
            {extra === "color" && (
              <div
                role="menu"
                data-extra-sub
                aria-label={t("selection.colorMenu")}
                className="grid-menu-sub"
              >
                {COLOR_TRAITS.map((c) => (
                  <BusyButton
                    key={c ?? "none"}
                    type="button"
                    role="menuitem"
                    className="grid-menu-item"
                    onClick={act(() =>
                      onBulk(target, { action: "set_color_trait", color_trait: c }, count),
                    )}
                  >
                    {t(`library.inspector.colorTraits.${c ?? "none"}`)}
                  </BusyButton>
                ))}
              </div>
            )}
            <BusyButton
              type="button"
              role="menuitem"
              className="grid-menu-item"
              onClick={() => setDating(true)}
            >
              {t("selection.setDateMenu")}
            </BusyButton>
            <BusyButton
              type="button"
              role="menuitem"
              className="grid-menu-item"
              onClick={act(() => onBulk(target, { action: "clear_manual_date" }, count))}
            >
              {t("library.inspector.restoreDate")}
            </BusyButton>
            <hr className="grid-menu-sep" />
          </>
        )}
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
      <ManualDateDialog
        open={dating}
        count={count}
        onOpenChange={(o) => {
          setDating(o);
          if (!o) onClose();
        }}
        onConfirm={(value, precision) =>
          onBulk?.(target, { action: "set_manual_date", value, precision }, count)
        }
      />
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
