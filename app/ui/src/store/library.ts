// Stato della Libreria: filtro corrente, ordinamento, dimensione miniature, selezione.
// Conservato tra i cambi di modulo (spec: la Libreria mantiene filtri, scorrimento e selezione).

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { LibraryFilter, SortKey } from "../lib/ipc";

export const emptyFilter = (): LibraryFilter => ({
  sourceIds: [],
  catalogId: null,
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
});

interface LibraryState {
  filter: LibraryFilter;
  sort: SortKey;
  /** 1–5: lato della miniatura = 100 + 40·(n−1) px */
  thumbSize: number;
  selected: number[];
  /** Ultimo elemento cliccato (ancora per Shift+clic) e mostrato nell'ispettore. */
  focused: number | null;
  scrollTop: number;
  setFilter: (f: Partial<LibraryFilter>) => void;
  resetFilter: () => void;
  setSort: (s: SortKey) => void;
  setThumbSize: (n: number) => void;
  setSelection: (ids: number[], focused?: number | null) => void;
  setScrollTop: (y: number) => void;
}

export const thumbPx = (n: number) => 100 + 40 * (Math.min(5, Math.max(1, n)) - 1);

export const useLibrary = create<LibraryState>()(
  persist(
    (set) => ({
      filter: emptyFilter(),
      sort: "captured_desc",
      thumbSize: 3,
      selected: [],
      focused: null,
      scrollTop: 0,
      setFilter: (f) => set((s) => ({ filter: { ...s.filter, ...f }, scrollTop: 0 })),
      resetFilter: () => set({ filter: emptyFilter(), scrollTop: 0 }),
      setSort: (sort) => set({ sort, scrollTop: 0 }),
      setThumbSize: (thumbSize) => set({ thumbSize }),
      setSelection: (selected, focused) =>
        set((s) => ({ selected, focused: focused === undefined ? s.focused : focused })),
      setScrollTop: (scrollTop) => set({ scrollTop }),
    }),
    {
      name: "photo-cat.library",
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({ sort: s.sort, thumbSize: s.thumbSize }),
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
): number[] {
  const { cols, cell, rowHeight, gap, pad } = layout;
  return order.filter((_, i) => {
    const x = pad + (i % cols) * (cell + gap),
      y = pad + Math.floor(i / cols) * rowHeight;
    return (
      x < box.x + box.w && x + cell > box.x && y < box.y + box.h && y + rowHeight - gap > box.y
    );
  });
}
