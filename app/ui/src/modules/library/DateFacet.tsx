// Lista Anno della barra laterale come gerarchia: sotto l'anno scelto i mesi
// di quell'anno, sotto il mese scelto i giorni di quel mese, con i conteggi.
import type { ReactNode } from "react";
import type { FacetList as FacetListData, FacetValue } from "../../lib/ipc";
import { fmtPeriod } from "../../lib/format";
import { useLibrary } from "../../store/library";
import { datePath, toggleDate } from "./facets";
import { FacetList } from "./FacetList";
import { FacetRow } from "./FacetValuesList";

const short = (v: string) => fmtPeriod(v, true);

export function DateFacet({
  years,
  months,
  days,
}: {
  years: FacetListData;
  months: FacetListData | null;
  days: FacetListData | null;
}) {
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const path = datePath(filter);
  const onToggle = (v: string) => setFilter(toggleDate(filter, v));
  const rows = (
    list: FacetListData | null,
    chosen: string | null,
    nested?: (v: FacetValue) => ReactNode,
  ) =>
    (list?.values ?? []).map((v) => (
      <div key={v.value}>
        <FacetRow value={v} checked={chosen === v.value} onToggle={onToggle} render={short} />
        {chosen === v.value && nested?.(v)}
      </div>
    ));
  const nest = (children: ReactNode) => <div className="facet-date-nest">{children}</div>;
  return (
    <FacetList
      facetKey={{ kind: "year" }}
      list={years}
      below={(y) =>
        y.value === path.year && months
          ? nest(rows(months, path.month, () => (days ? nest(rows(days, path.day)) : null)))
          : null
      }
    />
  );
}
