import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { Icon } from "../../components/icons";
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
  query: shared = "",
}: {
  facetKey: FacetKey;
  list: FacetListData;
  /** Ricerca comune a tutti i filtri della barra: mostra solo i valori corrispondenti. */
  query?: string;
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
  const searching = shared.trim().length > 0;
  const query = useDebounced(searching ? shared.trim() : search);
  const full = useFacetValues(facetKey, filter, query, "count", render, expanded || searching);
  const selected = selectedOf(facetKey, filter);
  const onToggle = (v: string) => setFilter(toggleIn(facetKey, filter, v));
  const top = list.values.slice(0, COMPACT);
  // Le voci selezionate restano visibili anche se non sono tra le principali.
  const pinned = list.values.filter(
    (v) => selected.includes(v.value) && !top.some((x) => x.value === v.value),
  );
  const shown = [...pinned, ...top];
  const hidden = list.total > top.length;
  if (searching) {
    // Con la ricerca comune: solo i gruppi con risultati, senza elenco compatto.
    if (full.query.data !== undefined && full.values.length === 0) return null;
    return (
      <div className="facet-block">
        <div className="facet-title">{title}</div>
        <FacetValuesList
          values={full.values}
          hasMore={full.hasMore}
          loadMore={full.loadMore}
          loading={full.loading}
          selected={selected}
          onToggle={onToggle}
          render={render}
          action={action}
          height={Math.min(240, 28 * Math.max(1, full.values.length))}
        />
      </div>
    );
  }
  return (
    <div className="facet-block">
      <div className="facet-title-row">
        <div className="facet-title">{title}</div>
        <BusyButton
          type="button"
          className="btn btn-ghost facet-icon-btn"
          title={t("library.facets.openWindowOf", { name: title })}
          aria-label={t("library.facets.openWindowOf", { name: title })}
          onClick={(e) => openFacetPanel(facetKey, e.currentTarget)}
        >
          <Icon.OpenExternal size={14} />
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
            className="btn btn-ghost facet-toggle"
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
              className="btn btn-ghost facet-toggle"
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
