// Lista Valutazione: ogni voce è un valore esatto (da 5 stelle a "Nessuna"). Scelta una voce
// compare il confronto "almeno / uguale a / al massimo", così "almeno ★★★★" sono due tocchi.
import { useTranslation } from "react-i18next";
import type { FacetList as FacetListData, RatingOp } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { FacetList } from "./FacetList";

const OPS: RatingOp[] = ["at_least", "equal", "at_most"];

export function RatingFacet({ list, query }: { list: FacetListData; query?: string }) {
  const { t } = useTranslation();
  const rating = useLibrary((s) => s.filter.rating);
  const setFilter = useLibrary((s) => s.setFilter);
  return (
    <div>
      <FacetList facetKey={{ kind: "rating" }} list={list} query={query} />
      {rating && (
        <select
          className="field"
          aria-label={t("library.facets.ratingOp")}
          value={rating.op}
          onChange={(e) => setFilter({ rating: { ...rating, op: e.target.value as RatingOp } })}
        >
          {OPS.map((op) => (
            <option key={op} value={op}>
              {t(`library.facets.ratingOps.${op}`)}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
