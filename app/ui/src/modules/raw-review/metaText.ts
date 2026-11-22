// Testi dei metadati di una variante (InfoSidebar e intestazioni dei pannelli, task 3.3).
// Il backend manda chiavi stabili per sezioni ed etichette e, per i valori enumerati,
// `valueKey`; il valore `value` (italiano, come in FujiView) resta il fallback quando una
// chiave non ha traduzione.
import type { TFunction } from "i18next";
import type { MetaRow, MetaSection, Recipe } from "../../lib/ipc";

export function metaSectionTitle(t: TFunction, section: Pick<MetaSection, "title">): string {
  return t(`rawReviewMeta.sections.${section.title}`, { defaultValue: section.title });
}

export function metaLabel(t: TFunction, row: Pick<MetaRow, "label">): string {
  return t(`rawReviewMeta.labels.${row.label}`, { defaultValue: row.label });
}

export function metaValue(t: TFunction, row: Pick<MetaRow, "value" | "valueKey">): string {
  if (!row.valueKey) return row.value;
  return t(`rawReviewMeta.values.${row.valueKey}`, { defaultValue: row.value });
}

/** Riga breve della ricetta ("Classic Chrome · DR200 · Auto") nella lingua dell'app. */
export function recipeSummary(
  t: TFunction,
  recipe: Pick<
    Recipe,
    "summary" | "film" | "filmKey" | "dynamicRange" | "whiteBalance" | "whiteBalanceKey"
  >,
): string {
  const parts = [
    recipe.film && metaValue(t, { value: recipe.film, valueKey: recipe.filmKey }),
    recipe.dynamicRange,
    recipe.whiteBalance &&
      metaValue(t, { value: recipe.whiteBalance, valueKey: recipe.whiteBalanceKey }),
  ].filter((p): p is string => !!p);
  return parts.length ? parts.join(" · ") : recipe.summary;
}
