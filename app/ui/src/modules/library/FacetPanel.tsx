// Pannelli flottanti delle liste di filtro: ricerca, ordinamento, numero di
// voci, elenco completo con caselle e "Deseleziona tutto". Leggono e
// scrivono il filtro della libreria, quindi restano allineati a barra
// laterale e griglia.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FloatingPanel } from "../../components/floating/FloatingPanel";
import type { FacetKey, FacetSort, FacetValue } from "../../lib/ipc";
import { useFloating } from "../../store/floating";
import { useLibrary } from "../../store/library";
import {
  clearIn,
  facetFromId,
  selectedOf,
  toggleIn,
  useDebounced,
  useFacetNames,
  useFacetValues,
} from "./facets";
import { FacetValuesList } from "./FacetValuesList";
import { RenamePlaceDialog } from "./RenamePlaceDialog";

export function FacetPanel({ facetKey }: { facetKey: FacetKey }) {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const { title, render } = useFacetNames(facetKey);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<FacetSort>("count");
  const [renaming, setRenaming] = useState<FacetValue | null>(null);
  const query = useDebounced(search);
  const list = useFacetValues(facetKey, filter, query, sort, render);
  const selected = selectedOf(facetKey, filter);
  const action =
    facetKey.kind === "place"
      ? (f: FacetValue) => (
          <button
            type="button"
            className="btn btn-ghost"
            style={{ padding: "0 4px", fontSize: 12 }}
            title={t("place.rename")}
            aria-label={t("place.renameOf", { name: f.value })}
            onClick={() => setRenaming(f)}
          >
            ✎
          </button>
        )
      : undefined;
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 8,
        padding: 10,
        flex: 1,
        minHeight: 0,
      }}
    >
      <input
        className="field"
        type="search"
        aria-label={t("library.facets.searchOf", { name: title })}
        placeholder={t("library.facets.searchOf", { name: title })}
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
        <select
          className="field"
          aria-label={t("library.facets.sort")}
          value={sort}
          onChange={(e) => setSort(e.target.value as FacetSort)}
        >
          <option value="count">{t("library.facets.sortCount")}</option>
          <option value="name">{t("library.facets.sortName")}</option>
        </select>
        <span className="muted" role="status">
          {t("library.facets.count", { count: list.total })}
        </span>
        <button
          type="button"
          className="btn btn-ghost"
          style={{ marginLeft: "auto" }}
          disabled={selected.length === 0}
          onClick={() => setFilter(clearIn(facetKey, filter))}
        >
          {t("library.facets.clearAll")}
        </button>
      </div>
      {renaming && (
        <RenamePlaceDialog
          key={renaming.value}
          from={renaming.value}
          count={renaming.count}
          places={list.values}
          onClose={() => setRenaming(null)}
        />
      )}
      <FacetValuesList
        values={list.values}
        hasMore={list.hasMore}
        loadMore={list.loadMore}
        loading={list.loading}
        selected={selected}
        onToggle={(v) => setFilter(toggleIn(facetKey, filter, v))}
        render={render}
        action={action}
        fill
      />
    </div>
  );
}

function FacetWindow({ id, facetKey }: { id: string; facetKey: FacetKey }) {
  const { title } = useFacetNames(facetKey);
  return (
    <FloatingPanel id={id} title={title}>
      <FacetPanel facetKey={facetKey} />
    </FloatingPanel>
  );
}

/** Pannelli dei filtri aperti: visibili solo nel modulo Libreria. */
export function FacetPanels() {
  const panels = useFloating((s) => s.panels);
  return (
    <>
      {Object.entries(panels)
        .filter(([, p]) => p.open)
        .map(([id]) => {
          const key = facetFromId(id);
          return key ? <FacetWindow key={id} id={id} facetKey={key} /> : null;
        })}
    </>
  );
}
