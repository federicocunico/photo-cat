// Lista di un filtro nella barra laterale: voci principali più quelle
// selezionate, "Mostra tutti (N)" con ricerca sul posto e "Apri in finestra".
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { FacetKey, FacetList as FacetListData } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import {
  COMPACT,
  openFacetPanel,
  selectedOf,
  toggleIn,
  useDebounced,
  useFacetNames,
  useFacetValues,
} from "./facets";
import { FacetRow, FacetValuesList, type RowAction } from "./FacetValuesList";

export function FacetList({
  facetKey,
  list,
  action,
}: {
  facetKey: FacetKey;
  list: FacetListData;
  action?: RowAction;
}) {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const { title, render } = useFacetNames(facetKey);
  const [expanded, setExpanded] = useState(false);
  const [search, setSearch] = useState("");
  const query = useDebounced(search);
  const full = useFacetValues(facetKey, filter, query, "count", render, expanded);
  const selected = selectedOf(facetKey, filter);
  const onToggle = (v: string) => setFilter(toggleIn(facetKey, filter, v));
  const top = list.values.slice(0, COMPACT);
  // Le voci selezionate restano visibili anche se non sono tra le principali.
  const pinned = list.values.filter(
    (v) => selected.includes(v.value) && !top.some((x) => x.value === v.value),
  );
  const shown = [...pinned, ...top];
  const hidden = list.total > top.length;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        <div style={{ fontWeight: 600, flex: 1 }}>{title}</div>
        <button
          type="button"
          className="btn btn-ghost"
          style={{ padding: "0 4px", fontSize: 12 }}
          title={t("library.facets.openWindowOf", { name: title })}
          aria-label={t("library.facets.openWindowOf", { name: title })}
          onClick={(e) => openFacetPanel(facetKey, e.currentTarget)}
        >
          ⧉
        </button>
      </div>
      {expanded ? (
        <>
          <input
            className="field"
            type="search"
            autoFocus
            aria-label={t("library.facets.searchOf", { name: title })}
            placeholder={t("library.facets.searchOf", { name: title })}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <FacetValuesList
            values={full.values}
            hasMore={full.hasMore}
            loadMore={full.loadMore}
            loading={full.loading}
            selected={selected}
            onToggle={onToggle}
            render={render}
            action={action}
            height={360}
          />
          <button
            type="button"
            className="btn btn-ghost"
            style={{ alignSelf: "flex-start", padding: "0 4px" }}
            onClick={() => {
              setExpanded(false);
              setSearch("");
            }}
          >
            {t("library.facets.showLess")}
          </button>
        </>
      ) : (
        <>
          {shown.map((v) => (
            <FacetRow
              key={v.value}
              value={v}
              checked={selected.includes(v.value)}
              onToggle={onToggle}
              render={render}
              action={action}
            />
          ))}
          {hidden && (
            <button
              type="button"
              className="btn btn-ghost"
              style={{ alignSelf: "flex-start", padding: "0 4px" }}
              onClick={() => setExpanded(true)}
            >
              {t("library.facets.showAll", { n: list.total })}
            </button>
          )}
        </>
      )}
    </div>
  );
}
