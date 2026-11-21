// Lista Anno della barra laterale come gerarchia: sotto l'anno scelto i mesi
// di quell'anno, sotto il mese scelto i giorni di quel mese, con i conteggi.
import type { ReactNode } from "react";
import type { FacetList as FacetListData, FacetValue } from "../../lib/ipc";
import { fmtPeriod } from "../../lib/format";
import { useLibrary } from "../../store/library";
import { useTranslation } from "react-i18next";
import { NO_MONTH, datePath, toggleDate } from "./facets";
import { FacetList } from "./FacetList";
import { FacetRow } from "./FacetValuesList";

export function DateFacet({
  years,
  months,
  days,
}: {
  years: FacetListData;
  months: FacetListData | null;
  days: FacetListData | null;
}) {
  const { t } = useTranslation();
  const short = (v: string) =>
    NO_MONTH.test(v) ? t("library.facets.noMonth") : fmtPeriod(v, true);
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const path = datePath(filter);
  const noMonth = filter.dateNoMonth ?? false;
  const onToggle = (v: string) => {
    if (NO_MONTH.test(v)) {
      // "Anno senza mese": le foto con data manuale di solo anno, dentro l'anno scelto.
      if (noMonth) setFilter({ dateNoMonth: false });
      else
        setFilter({
          ...toggleDate({ dateFrom: null, dateTo: null }, v.slice(0, 4)),
          dateNoMonth: true,
        });
      return;
    }
    setFilter({ ...toggleDate(filter, v), ...(noMonth ? { dateNoMonth: false } : {}) });
  };
  const rows = (
    list: FacetListData | null,
    chosen: string | null,
    nested?: (v: FacetValue) => ReactNode,
  ) =>
    (list?.values ?? []).map((v) => (
      <div key={v.value}>
        <FacetRow
          value={v}
          checked={NO_MONTH.test(v.value) ? noMonth : chosen === v.value}
          onToggle={onToggle}
          render={short}
        />
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
