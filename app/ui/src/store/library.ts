// Stato della Libreria: filtro corrente, ordinamento, dimensione miniature, selezione.
// Conservato tra i cambi di modulo (spec: la Libreria mantiene filtri, scorrimento e selezione).

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { mirroredStorage } from "../lib/uiPrefs";
import type { LibraryFilter, SelectionBase, SortKey, Target } from "../lib/ipc";

export const emptyFilter = (): LibraryFilter => ({
  sourceIds: [],
  catalogId: null,
  excludedFrom: null,
  matchingOnly: true,
  labels: [],
  personIds: [],
  people: null,
  places: [],
  dateFrom: null,
  dateTo: null,
  formats: [],
  tags: [],
  text: null,
  status: null,
  duplicatesOnly: false,
  rule: null,
  folder: null,
  ids: null,
  similarOnly: false,
  groupSimilar: false,
});

/**
 * Selezione come insieme: elementi scelti uno per uno, oppure "tutti i risultati del filtro"
 * (Seleziona tutto) meno le eccezioni. Con 100.000 risultati non si carica mai l'elenco degli id:
 * `maxId` limita la selezione agli elementi presenti in quell'istante, `total` è il loro numero.
 */
export type Selection =
  | { mode: "ids"; ids: ReadonlySet<number> }
  | {
      mode: "filter";
      filter: LibraryFilter;
      maxId: number;
      total: number;
      except: ReadonlySet<number>;
    };

export const idsSelection = (ids: Iterable<number> = []): Selection => ({
  mode: "ids",
  ids: new Set(ids),
});

/** Numero di elementi selezionati. */
export const selCount = (s: Selection) =>
  s.mode === "ids" ? s.ids.size : Math.max(0, s.total - s.except.size);

/** L'elemento è selezionato (O(1)). */
export const selHas = (s: Selection, id: number) =>
  s.mode === "ids" ? s.ids.has(id) : id <= s.maxId && !s.except.has(id);

/** Id espliciti, o `null` per una selezione per filtro (va passata come `Target`). */
export const selIds = (s: Selection): number[] | null => (s.mode === "ids" ? [...s.ids] : null);

/** Bersaglio dei comandi bulk: il backend risolve gli id a blocchi. */
export const selTarget = (s: Selection): Target =>
  s.mode === "ids"
    ? { kind: "ids", ids: [...s.ids] }
    : { kind: "filter", filter: s.filter, maxId: s.maxId, except: [...s.except] };

/** Aggiunge (`on`) o toglie elementi dalla selezione, in entrambe le modalità. */
export function selSet(s: Selection, ids: Iterable<number>, on: boolean): Selection {
  if (s.mode === "ids") {
    const next = new Set(s.ids);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
    }
    return { mode: "ids", ids: next };
  }
  const except = new Set(s.except);
  for (const id of ids) {
    if (on) except.delete(id);
    else except.add(id);
  }
  return { ...s, except };
}

interface LibraryState {
  filter: LibraryFilter;
  sort: SortKey;
  /** 1–5: lato della miniatura = 100 + 40·(n−1) px */
  thumbSize: number;
  selection: Selection;
  /** Ultimo elemento cliccato (ancora per Shift+clic) e mostrato nell'ispettore. */
  focused: number | null;
  scrollTop: number;
  /** Il filtro per cartella include le sottocartelle. */
  folderRecursive: boolean;
  /** "Raggruppa foto simili": un elemento per gruppo nella griglia (preferenza persistita). */
  groupSimilar: boolean;
  setGroupSimilar: (v: boolean) => void;
  setFolderRecursive: (v: boolean) => void;
  setFilter: (f: Partial<LibraryFilter>) => void;
  resetFilter: () => void;
  setSort: (s: SortKey) => void;
  setThumbSize: (n: number) => void;
  /** Selezione esplicita (clic, intervallo, rettangolo). */
  setSelection: (ids: number[], focused?: number | null) => void;
  /** Sostituisce la selezione (es. aggiunte o eccezioni in una selezione per filtro). */
  replaceSelection: (s: Selection, focused?: number | null) => void;
  /** "Seleziona tutto" sul filtro corrente. */
  selectAll: (base: SelectionBase) => void;
  /** Inversione nel filtro corrente: gli id scelti diventano le eccezioni e viceversa. */
  invertSelection: (base: SelectionBase) => void;
  setScrollTop: (y: number) => void;
}

export const thumbPx = (n: number) => 100 + 40 * (Math.min(5, Math.max(1, n)) - 1);

/** Un cambio di filtro rende obsoleta una selezione "tutti i risultati" del filtro precedente. */
const keepSelection = (s: Selection) => (s.mode === "filter" ? idsSelection() : s);

export const useLibrary = create<LibraryState>()(
  persist(
    (set) => ({
      filter: emptyFilter(),
      sort: "captured_desc",
      thumbSize: 3,
      selection: idsSelection(),
      focused: null,
      scrollTop: 0,
      folderRecursive: true,
      groupSimilar: false,
      setGroupSimilar: (groupSimilar) => set({ groupSimilar, scrollTop: 0 }),
      setFolderRecursive: (folderRecursive) =>
        set((s) => ({
          folderRecursive,
          filter: s.filter.folder
            ? { ...s.filter, folder: { ...s.filter.folder, recursive: folderRecursive } }
            : s.filter,
          selection: s.filter.folder ? keepSelection(s.selection) : s.selection,
        })),
      setFilter: (f) =>
        set((s) => {
          const filter = { ...s.filter, ...f };
          if (f.catalogId != null || (f.catalogId === null && f.excludedFrom === undefined))
            filter.excludedFrom = null;
          return {
            filter,
            sort:
              filter.excludedFrom == null && s.sort === "excluded_at" ? "captured_desc" : s.sort,
            scrollTop: 0,
            selection: keepSelection(s.selection),
          };
        }),
      resetFilter: () =>
        set((s) => ({
          filter: emptyFilter(),
          sort: s.sort === "excluded_at" ? "captured_desc" : s.sort,
          scrollTop: 0,
          selection: keepSelection(s.selection),
        })),
      setSort: (sort) => set({ sort, scrollTop: 0 }),
      setThumbSize: (thumbSize) => set({ thumbSize }),
      setSelection: (ids, focused) =>
        set((s) => ({
          selection: idsSelection(ids),
          focused: focused === undefined ? s.focused : focused,
        })),
      replaceSelection: (selection, focused) =>
        set((s) => ({ selection, focused: focused === undefined ? s.focused : focused })),
      selectAll: (base) =>
        set((s) => ({
          selection: {
            mode: "filter",
            filter: s.filter,
            maxId: base.maxId,
            total: base.count,
            except: new Set(),
          },
        })),
      invertSelection: (base) =>
        set((s) => ({
          selection:
            s.selection.mode === "ids"
              ? {
                  mode: "filter",
                  filter: s.filter,
                  maxId: base.maxId,
                  total: base.count,
                  except: s.selection.ids,
                }
              : idsSelection(s.selection.except),
        })),
      setScrollTop: (scrollTop) => set({ scrollTop }),
    }),
    {
      name: "photo-cat.library",
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({
        // Il filtro non è persistito: al riavvio si apre la libreria generale.
        sort: s.sort === "excluded_at" ? "captured_desc" : s.sort,
        thumbSize: s.thumbSize,
        folderRecursive: s.folderRecursive,
        groupSimilar: s.groupSimilar,
      }),
    },
  ),
);

/** Clic su un elemento: singolo, Ctrl/Cmd (aggiungi/togli), Shift (intervallo dall'ancora). */
export function clickSelect(
  order: number[],
  current: number[],
  anchor: number | null,
  id: number,
  mods: { ctrl: boolean; shift: boolean },
): number[] {
  if (mods.shift && anchor !== null) {
    const a = order.indexOf(anchor);
    const b = order.indexOf(id);
    if (a >= 0 && b >= 0) {
      const range = order.slice(Math.min(a, b), Math.max(a, b) + 1);
      return mods.ctrl ? Array.from(new Set([...current, ...range])) : range;
    }
  }
  if (mods.ctrl) return current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
  return [id];
}

/** Rettangolo nello spazio del contenuto, indipendente dai nodi virtualizzati. */
export function rectangleSelect(
  order: number[],
  box: { x: number; y: number; w: number; h: number },
  layout: { cols: number; cell: number; rowHeight: number; gap: number; pad: number },
  /** Posizione globale del primo elemento di `order` (griglia a finestra). */
  offset = 0,
): number[] {
  const { cols, cell, rowHeight, gap, pad } = layout;
  return order.filter((_, k) => {
    const i = k + offset;
    const x = pad + (i % cols) * (cell + gap),
      y = pad + Math.floor(i / cols) * rowHeight;
    return (
      x < box.x + box.w && x + cell > box.x && y < box.y + box.h && y + rowHeight - gap > box.y
    );
  });
}
