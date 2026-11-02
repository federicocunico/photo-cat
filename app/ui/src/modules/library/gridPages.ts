// Pagine della griglia a memoria limitata: al massimo MAX_PAGES pagine vicine alla vista.
// Scorrendo in avanti si scaricano le prime, tornando indietro si ricaricano dal cursore del
// primo elemento; un salto lontano (barra di scorrimento) riparte da un offset (`anchor`).

import { useInfiniteQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import {
  commands,
  isTauri,
  type LibraryFilter,
  type Page,
  type PageAt,
  type SortKey,
} from "../../lib/ipc";

export const PAGE_SIZE = 300;
export const MAX_PAGES = 5;
/** Le chiavi della libreria non più mostrate (altri filtri, salti) si liberano presto. */
export const GRID_GC_MS = 30_000;

/** Inizio di una pagina: `offset` è la posizione globale del suo primo elemento. */
export type GridPageParam =
  | { at: PageAt | null; offset: number }
  /** Pagina che finisce a `end` (escluso): la si legge all'indietro dal cursore. */
  | { at: PageAt; end: number };

export type GridPage = Page & { offset: number };

export const gridKey = (filter: LibraryFilter, sort: SortKey, anchor: number) =>
  ["library", filter, sort, anchor] as const;

export function initialParam(anchor: number): GridPageParam {
  return anchor > 0
    ? { at: { kind: "offset", offset: anchor }, offset: anchor }
    : { at: null, offset: 0 };
}

export function useGridPages(filter: LibraryFilter, sort: SortKey, anchor: number) {
  const pages = useInfiniteQuery({
    queryKey: gridKey(filter, sort, anchor),
    queryFn: async ({ pageParam }): Promise<GridPage> => {
      const page = await commands.libraryPage(filter, sort, pageParam.at, PAGE_SIZE);
      const offset = "end" in pageParam ? pageParam.end - page.items.length : pageParam.offset;
      return { ...page, offset: Math.max(0, offset) };
    },
    initialPageParam: initialParam(anchor),
    getNextPageParam: (last): GridPageParam | undefined =>
      last.next
        ? { at: { kind: "after", cursor: last.next }, offset: last.offset + last.items.length }
        : undefined,
    getPreviousPageParam: (first): GridPageParam | undefined =>
      first.prev && first.offset > 0
        ? { at: { kind: "before", cursor: first.prev }, end: first.offset }
        : undefined,
    maxPages: MAX_PAGES,
    gcTime: GRID_GC_MS,
    enabled: isTauri(),
  });
  const start = pages.data?.pages[0]?.offset ?? anchor;
  const items = useMemo(() => pages.data?.pages.flatMap((p) => p.items) ?? [], [pages.data]);
  return { pages, start, items };
}

/**
 * Cosa caricare per mostrare gli elementi `[from, to)`, data la finestra `[start, start + len)`.
 * `margin` elementi di anticipo prima dei bordi della finestra.
 */
export function windowAction(
  view: { from: number; to: number },
  win: { start: number; len: number; hasNext: boolean; hasPrev: boolean },
  margin: number,
): "next" | "prev" | { seek: number } | null {
  const end = win.start + win.len;
  // Vista lontana dalla finestra: si riparte da lì invece di scorrere pagina per pagina.
  if (view.to < win.start - PAGE_SIZE || view.from > end + PAGE_SIZE) {
    return { seek: Math.floor(view.from / PAGE_SIZE) * PAGE_SIZE };
  }
  if (win.hasNext && view.to > end - margin) return "next";
  if (win.hasPrev && view.from < win.start + margin) return "prev";
  return null;
}
