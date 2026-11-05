import i18n from "i18next";
import type { LibraryFilter } from "../../lib/ipc";
import { fmtPeriod } from "../../lib/format";
import { datePath } from "./facets";

/** Filtri attivi come chip rimovibili: nome e modifica che lo toglie. */
export function activeChips(
  f: LibraryFilter,
  t: (k: string, o?: object) => string,
  label?: (group: string, value: string) => string,
  /** Nome di una persona per il chip del filtro persone. */
  person?: (id: number) => string | undefined,
  /** Nome di un gruppo di etichette per il chip della confidenza. */
  group?: (group: string) => string,
) {
  const chips: Array<{ name: string; clear: Partial<LibraryFilter> }> = [];
  for (const l of f.labels ?? []) {
    for (const v of l.values) {
      chips.push({
        name: label ? label(l.group, v) : t(`labels.${v}`, { defaultValue: v.replace(/_/g, " ") }),
        clear: {
          labels: (f.labels ?? [])
            .map((x) =>
              x.group === l.group ? { ...x, values: x.values.filter((y) => y !== v) } : x,
            )
            .filter((x) => x.values.length > 0),
        },
      });
    }
  }
  for (const id of f.personIds ?? [])
    chips.push({
      name: person?.(id) ?? t("selection.unnamed"),
      clear: { personIds: (f.personIds ?? []).filter((x) => x !== id) },
    });
  for (const p of f.places ?? [])
    chips.push({ name: p, clear: { places: (f.places ?? []).filter((x) => x !== p) } });
  for (const x of f.formats ?? [])
    chips.push({
      name: x.toUpperCase(),
      clear: { formats: (f.formats ?? []).filter((y) => y !== x) },
    });
  for (const x of f.tags ?? [])
    chips.push({ name: `#${x}`, clear: { tags: (f.tags ?? []).filter((y) => y !== x) } });
  const date = datePath(f);
  const chosen = date.day ?? date.month ?? date.year;
  if (chosen) chips.push({ name: fmtPeriod(chosen), clear: { dateFrom: null, dateTo: null } });
  if (f.text) chips.push({ name: `“${f.text}”`, clear: { text: null } });
  if (f.analysis)
    chips.push({
      name: t("filterChips.analysis", {
        stage: t(`filterChips.stage.${f.analysis.stage}`),
        state: t(`filterChips.state.${f.analysis.state}`),
      }),
      clear: { analysis: null },
    });
  if (f.labelScore) {
    const n = (x: number | null) => (x ?? 0).toLocaleString(i18n.language, { maximumFractionDigits: 2 });
    chips.push({
      name: t("filterChips.score", {
        group: group ? group(f.labelScore.grp) : f.labelScore.grp,
        min: n(f.labelScore.min),
        max: n(f.labelScore.max),
      }),
      clear: { labelScore: null },
    });
  }
  if (f.similarSize)
    chips.push({
      name: t(`filterChips.similarSize.${f.similarSize}`),
      clear: { similarSize: null },
    });
  if (f.geoBbox)
    chips.push({
      name: t("filterChips.area", { label: f.geoBbox.label }),
      clear: { geoBbox: null },
    });
  if (f.similarOnly) chips.push({ name: t("similar.chip"), clear: { similarOnly: false } });
  if (f.folder?.path) {
    const name = f.folder.path.split(/[\\/]/).filter(Boolean).pop() ?? f.folder.path;
    chips.push({
      name: t(f.folder.recursive ? "folders.chip" : "folders.chipExact", { name }),
      clear: { folder: null },
    });
  }
  if ((f.sourceIds ?? []).length)
    chips.push({ name: t("library.sources"), clear: { sourceIds: [] } });
  if (f.ids)
    chips.push({ name: t("sources.idsChip", { count: f.ids.length }), clear: { ids: null } });
  if (f.catalogId != null) chips.push({ name: t("library.catalogs"), clear: { catalogId: null } });
  return chips;
}
