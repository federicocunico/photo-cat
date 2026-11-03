// Barra unica della Libreria (desktop-shell, "Contesto della vista in Libreria"):
// percorso, conteggio, filtri attivi, menu Ordina e Vista, interruttore dell'ispettore.
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { Button, IconButton, Menu, Popover } from "../../components/ui";
import "./library.css";
import { commands, isTauri, type SortKey } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLibraryCount } from "../../lib/latest";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { activeChips } from "./chips";
import { useLabelName } from "./labelName";
import { catalogLabel } from "../catalogs/systemCatalogs";
import { SelectMenu } from "./SelectionBar";

const SORTS: SortKey[] = ["captured_desc", "captured_asc", "name_asc", "size_desc", "added_desc"];

const MOD = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "") ? "⌘" : "Ctrl";

/** Percorso: Tutte le foto › catalogo › Escluse. */
function Breadcrumb() {
  const { t } = useTranslation();
  const { filter, setFilter, setSelection, setSort } = useLibrary();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const id = filter.excludedFrom ?? filter.catalogId;
  const summary = id == null ? undefined : catalogs.data?.find((c) => c.catalog.id === id);
  const excluded = filter.excludedFrom != null;
  const toAll = () => {
    setSelection([], null);
    setFilter({ catalogId: null, excludedFrom: null, matchingOnly: true });
    if (excluded) setSort("captured_desc");
  };
  const toCatalog = () => {
    setSelection([], null);
    setFilter({ catalogId: id, excludedFrom: null, matchingOnly: true });
    setSort("captured_desc");
  };
  const all = t("library.header.all");
  return (
    <nav aria-label={t("library.header.path")} className="lib-path">
      {id == null ? (
        <strong aria-current="page">{all}</strong>
      ) : (
        <button type="button" className="lib-path-link" onClick={toAll}>
          {all}
        </button>
      )}
      {summary && (
        <>
          <span className="muted" aria-hidden="true">
            ›
          </span>
          {excluded ? (
            <button type="button" className="lib-path-link" onClick={toCatalog}>
              {catalogLabel(summary.catalog, t)}
            </button>
          ) : (
            <strong aria-current="page">{catalogLabel(summary.catalog, t)}</strong>
          )}
        </>
      )}
      {summary && excluded && (
        <>
          <span className="muted" aria-hidden="true">
            ›
          </span>
          <strong aria-current="page">{t("library.header.excluded")}</strong>
        </>
      )}
    </nav>
  );
}

/** "Escluse (N)" nel catalogo aperto; nelle escluse di un catalogo smart, anche le non corrispondenti. */
function ExclusionsControl() {
  const { t } = useTranslation();
  const { filter, setFilter, setSelection, setSort } = useLibrary();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const id = filter.excludedFrom ?? filter.catalogId;
  const summary = id == null ? undefined : catalogs.data?.find((c) => c.catalog.id === id);
  if (!summary || summary.catalog.system) return null;
  if (filter.excludedFrom == null)
    return (
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          setSelection([], null);
          setFilter({ catalogId: null, excludedFrom: id, matchingOnly: true });
          setSort("excluded_at");
        }}
      >
        {t("exclusions.view", { count: summary.excluded })}
      </Button>
    );
  if (summary.catalog.kind !== "smart") return null;
  return (
    <label className="lib-header-check">
      <input
        type="checkbox"
        checked={filter.matchingOnly === false}
        onChange={(e) => {
          setSelection([], null);
          setFilter({ matchingOnly: !e.target.checked });
        }}
      />
      {t("exclusions.all")}
    </label>
  );
}

function Count() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const count = useLibraryCount(filter);
  return (
    <span className="muted" data-testid="library-header-count">
      {count.isPending
        ? t("feedback.calculating")
        : count.isError
          ? t("feedback.error")
          : t("library.header.count", { count: count.data ?? 0, n: fmtInt(count.data ?? 0) })}
    </span>
  );
}

function Chips() {
  const { t } = useTranslation();
  const { filter, setFilter, resetFilter } = useLibrary();
  const names = useLabelName();
  const people = useQuery({
    queryKey: ["people"],
    queryFn: () => commands.listPeople(),
    enabled: isTauri() && (filter.personIds ?? []).length > 0,
  });
  const person = (id: number) => people.data?.find((p) => p.id === id)?.name ?? undefined;
  // Il catalogo aperto è già nel percorso: niente chip doppio.
  const chips = activeChips({ ...filter, catalogId: null }, t as never, names.label, person);
  if (chips.length === 0) return null;
  return (
    <div className="lib-chips">
      {chips.map((c) => (
        <button
          key={c.name}
          type="button"
          className="chip lib-chip"
          aria-label={t("library.removeFilter", { name: c.name })}
          onClick={() => setFilter(c.clear)}
        >
          {c.name} <Icon.Close size={12} aria-hidden="true" />
        </button>
      ))}
      {chips.length > 1 && (
        <Button variant="ghost" size="sm" onClick={resetFilter}>
          {t("library.clearFilters")}
        </Button>
      )}
    </div>
  );
}

function SortMenu() {
  const { t } = useTranslation();
  const { filter, sort, setSort } = useLibrary();
  const keys: SortKey[] = filter.excludedFrom != null ? ["excluded_at", ...SORTS] : SORTS;
  const name = (k: SortKey) =>
    k === "excluded_at" ? t("exclusions.sort") : t(`library.sortKeys.${k}`);
  return (
    <Menu
      align="end"
      trigger={
        <button type="button" className="btn btn-ghost btn-sm">
          {t("library.header.sortBy", { name: name(sort) })}
          <Icon.ChevronDown size={14} aria-hidden="true" />
        </button>
      }
      items={keys.map((k) => ({
        label: name(k),
        icon:
          k === sort ? (
            <Icon.Check size={14} aria-label={t("library.header.current")} />
          ) : (
            <span className="lib-sort-spacer" />
          ),
        onSelect: () => setSort(k),
      }))}
    />
  );
}

function ViewMenu() {
  const { t } = useTranslation();
  const { thumbSize, setThumbSize } = useLibrary();
  const groupSimilar = useLibrary((s) => s.groupSimilar);
  const setGroupSimilar = useLibrary((s) => s.setGroupSimilar);
  return (
    <Popover
      align="end"
      label={t("library.header.view")}
      trigger={
        <button type="button" className="btn btn-ghost btn-sm">
          {t("library.header.view")}
          <Icon.ChevronDown size={14} aria-hidden="true" />
        </button>
      }
    >
      <div className="lib-view">
        <label className="lib-view-row">
          {t("library.thumbSize")}
          <input
            type="range"
            min={1}
            max={5}
            value={thumbSize}
            onChange={(e) => setThumbSize(Number(e.target.value))}
          />
        </label>
        <label className="lib-view-row">
          <input
            type="checkbox"
            checked={groupSimilar}
            onChange={(e) => setGroupSimilar(e.target.checked)}
          />
          {t("similar.group")}
        </label>
      </div>
    </Popover>
  );
}

function InspectorToggle() {
  const { t } = useTranslation();
  const open = useLayout((s) => s.rightOpen);
  const toggle = useLayout((s) => s.toggleRight);
  const label = t(open ? "shell.collapseRight" : "shell.expandRight");
  return (
    <IconButton
      label={label}
      shortcut={`${MOD}+I`}
      aria-pressed={open}
      aria-keyshortcuts="Control+I"
      onClick={toggle}
      icon={<Icon.PanelRight size={18} />}
    />
  );
}

export function LibraryHeader() {
  return (
    <div className="lib-header">
      <Breadcrumb />
      <Count />
      <ExclusionsControl />
      <Chips />
      <div className="lib-header-actions">
        <SelectMenu />
        <SortMenu />
        <ViewMenu />
        <InspectorToggle />
      </div>
    </div>
  );
}
