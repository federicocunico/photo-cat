import type { LibraryFilter } from "../../lib/ipc";

/** Filtri attivi come chip rimovibili: nome e modifica che lo toglie. */
export function activeChips(f: LibraryFilter, t: (k: string, o?: object) => string) {
  const chips: Array<{ name: string; clear: Partial<LibraryFilter> }> = [];
  for (const l of f.labels ?? []) {
    for (const v of l.values) {
      chips.push({
        name: t(`labels.${v}`, { defaultValue: v.replace(/_/g, " ") }),
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
  for (const p of f.places ?? [])
    chips.push({ name: p, clear: { places: (f.places ?? []).filter((x) => x !== p) } });
  for (const x of f.formats ?? [])
    chips.push({
      name: x.toUpperCase(),
      clear: { formats: (f.formats ?? []).filter((y) => y !== x) },
    });
  for (const x of f.tags ?? [])
    chips.push({ name: `#${x}`, clear: { tags: (f.tags ?? []).filter((y) => y !== x) } });
  if (f.dateFrom)
    chips.push({ name: f.dateFrom.slice(0, 4), clear: { dateFrom: null, dateTo: null } });
  if (f.text) chips.push({ name: `“${f.text}”`, clear: { text: null } });
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
