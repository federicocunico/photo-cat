import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
// Lista di un filtro nella barra laterale: voci principali più quelle
// selezionate, "Mostra tutti (N)" con ricerca sul posto e "Apri in finestra".
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { FacetKey, FacetList as FacetListData, FacetValue } from "../../lib/ipc";
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
  below,
}: {
  facetKey: FacetKey;
  list: FacetListData;
  action?: RowAction;
  /** Contenuto sotto una voce della lista compatta (mesi sotto l'anno scelto). */
  below?: (v: FacetValue) => ReactNode;
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
        <BusyButton
          type="button"
          className="btn btn-ghost"
          style={{ padding: "0 4px", fontSize: 12 }}
          title={t("library.facets.openWindowOf", { name: title })}
          aria-label={t("library.facets.openWindowOf", { name: title })}
          onClick={(e) => openFacetPanel(facetKey, e.currentTarget)}
        >
          ⧉
        </BusyButton>
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
          <QueryView
            query={{ ...full.query, data: full.query.data === undefined ? undefined : full.values }}
            isEmpty={() => false}
          >
            {() => (
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
            )}
          </QueryView>
          <BusyButton
            type="button"
            className="btn btn-ghost"
            style={{ alignSelf: "flex-start", padding: "0 4px" }}
            onClick={() => {
              setExpanded(false);
              return setSearch("");
            }}
          >
            {t("library.facets.showLess")}
          </BusyButton>
        </>
      ) : (
        <>
          {shown.map((v) => (
            <div key={v.value}>
              <FacetRow
                value={v}
                checked={selected.includes(v.value)}
                onToggle={onToggle}
                render={render}
                action={action}
              />
              {below?.(v)}
            </div>
          ))}
          {hidden && (
            <BusyButton
              type="button"
              className="btn btn-ghost"
              style={{ alignSelf: "flex-start", padding: "0 4px" }}
              onClick={() => setExpanded(true)}
            >
              {t("library.facets.showAll", { n: list.total })}
            </BusyButton>
          )}
        </>
      )}
    </div>
  );
}
