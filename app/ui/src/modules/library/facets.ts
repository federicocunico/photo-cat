// Liste dei filtri della libreria: chiave, selezione nel filtro, nomi delle
// voci, caricamento a pagine dell'elenco completo e apertura in un pannello.
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type FacetKey,
  type FacetSort,
  type FacetValue,
  type LibraryFilter,
} from "../../lib/ipc";
import { openers, useFloating } from "../../store/floating";
import { useLabelName } from "./labelName";

export const PAGE = 100;
/** Voci mostrate nella lista compatta (più quelle selezionate). */
export const COMPACT = 8;

export const facetId = (key: FacetKey) =>
  key.kind === "label" ? `facet:label:${key.group}` : `facet:${key.kind}`;

export function facetFromId(id: string): FacetKey | null {
  if (id.startsWith("facet:label:")) return { kind: "label", group: id.slice(12) };
  const kind = id.slice(6);
  return ["place", "year", "format", "tag"].includes(kind) && id.startsWith("facet:")
    ? ({ kind } as FacetKey)
    : null;
}

const toggle = (list: string[], v: string) =>
  list.includes(v) ? list.filter((x) => x !== v) : [...list, v];

/** Voci selezionate nel filtro per questa lista. */
export function selectedOf(key: FacetKey, f: LibraryFilter): string[] {
  switch (key.kind) {
    case "label":
      return f.labels?.find((l) => l.group === key.group)?.values ?? [];
    case "place":
      return f.places ?? [];
    case "year":
      return f.dateFrom ? [f.dateFrom.slice(0, 4)] : [];
    case "format":
      return f.formats ?? [];
    case "tag":
      return f.tags ?? [];
  }
}

/** Modifica del filtro che seleziona o deseleziona una voce. */
export function toggleIn(key: FacetKey, f: LibraryFilter, v: string): Partial<LibraryFilter> {
  switch (key.kind) {
    case "label": {
      const values = toggle(selectedOf(key, f), v);
      const others = (f.labels ?? []).filter((l) => l.group !== key.group);
      return { labels: values.length ? [...others, { group: key.group, values }] : others };
    }
    case "place":
      return { places: toggle(f.places ?? [], v) };
    case "year":
      return (f.dateFrom ?? "").slice(0, 4) === v
        ? { dateFrom: null, dateTo: null }
        : { dateFrom: `${v}-01-01`, dateTo: `${v}-12-31` };
    case "format":
      return { formats: toggle(f.formats ?? [], v) };
    case "tag":
      return { tags: toggle(f.tags ?? [], v) };
  }
}

/** Modifica del filtro che toglie tutta la selezione della lista. */
export function clearIn(key: FacetKey, f: LibraryFilter): Partial<LibraryFilter> {
  switch (key.kind) {
    case "label":
      return { labels: (f.labels ?? []).filter((l) => l.group !== key.group) };
    case "place":
      return { places: [] };
    case "year":
      return { dateFrom: null, dateTo: null };
    case "format":
      return { formats: [] };
    case "tag":
      return { tags: [] };
  }
}

/** Titolo della lista e nome mostrato di ogni voce. */
export function useFacetNames(key: FacetKey) {
  const { t } = useTranslation();
  const names = useLabelName();
  const title =
    key.kind === "label"
      ? names.group(key.group)
      : t(
          {
            place: "library.facets.places",
            year: "library.facets.years",
            format: "library.facets.formats",
            tag: "library.facets.tags",
          }[key.kind],
        );
  const render = (v: string) =>
    key.kind === "label" ? names.label(key.group, v) : key.kind === "format" ? v.toUpperCase() : v;
  return { title, render };
}

/** Valore ritardato (ricerca mentre si scrive). */
export function useDebounced<T>(value: T, ms = 200) {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

/**
 * Elenco completo di una lista, a pagine. Per le catalogazioni AI in ordine
 * alfabetico si caricano tutte le voci del gruppo e si ordinano sul nome
 * mostrato (tradotto), che il backend non conosce.
 */
export function useFacetValues(
  key: FacetKey,
  filter: LibraryFilter,
  search: string,
  sort: FacetSort,
  render: (v: string) => string,
  enabled = true,
) {
  const byLabelName = key.kind === "label" && sort === "name";
  const pages = useInfiniteQuery({
    queryKey: ["facet-values", key, filter, search, sort],
    queryFn: ({ pageParam }) =>
      commands.libraryFacetValues(filter, key, search || null, sort, pageParam, PAGE),
    initialPageParam: 0,
    getNextPageParam: (last, all) => {
      const loaded = all.reduce((n, p) => n + p.values.length, 0);
      return loaded < last.total ? loaded : undefined;
    },
    enabled: enabled && isTauri() && !byLabelName,
    placeholderData: (prev) => prev,
  });
  const whole = useQuery({
    queryKey: ["facet-values", key, filter, search, sort, "all"],
    queryFn: () => commands.libraryFacetValues(filter, key, search || null, "count", 0, 1000),
    enabled: enabled && isTauri() && byLabelName,
    placeholderData: (prev) => prev,
  });
  if (byLabelName) {
    const values: FacetValue[] = [...(whole.data?.values ?? [])].sort((a, b) =>
      render(a.value).localeCompare(render(b.value), undefined, { sensitivity: "base" }),
    );
    return {
      values,
      total: whole.data?.total ?? 0,
      loading: whole.isLoading,
      hasMore: false,
      loadMore: () => {},
    };
  }
  return {
    values: pages.data?.pages.flatMap((p) => p.values) ?? [],
    total: pages.data?.pages[0]?.total ?? 0,
    loading: pages.isLoading,
    hasMore: pages.hasNextPage,
    loadMore: () => {
      if (pages.hasNextPage && !pages.isFetchingNextPage) void pages.fetchNextPage();
    },
  };
}

/** Apre (o porta in primo piano) il pannello flottante di una lista. */
export function openFacetPanel(key: FacetKey, opener: HTMLElement | null) {
  const id = facetId(key);
  if (opener) openers.set(id, opener);
  const count = Object.values(useFloating.getState().panels).filter((p) => p.open).length;
  useFloating.getState().open(id, { x: 300 + count * 24, y: 120 + count * 24, w: 320, h: 420 });
}
