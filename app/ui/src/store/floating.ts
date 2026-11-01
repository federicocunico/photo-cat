// Pannelli flottanti: aperti, posizione, dimensione e ordine di sovrapposizione.
// Posizione e dimensione restano tra un'apertura e l'altra e tra i riavvii.
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export interface PanelRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PanelState extends PanelRect {
  open: boolean;
}

export const MIN_W = 260;
export const MIN_H = 240;

interface FloatingState {
  panels: Record<string, PanelState>;
  /** Id dal più lontano al più vicino (l'ultimo è in primo piano). */
  order: string[];
  open: (id: string, initial: PanelRect) => void;
  close: (id: string) => void;
  setRect: (id: string, rect: Partial<PanelRect>) => void;
  front: (id: string) => void;
}

/** Elemento che ha aperto ogni pannello: il focus vi torna alla chiusura. */
export const openers = new Map<string, HTMLElement>();

/** Riconduce il rettangolo dentro la finestra, riducendolo se serve. */
export function fit(r: PanelRect, vw = window.innerWidth, vh = window.innerHeight): PanelRect {
  const w = Math.max(Math.min(r.w, vw), Math.min(MIN_W, vw));
  const h = Math.max(Math.min(r.h, vh), Math.min(MIN_H, vh));
  return {
    w,
    h,
    x: Math.min(Math.max(0, r.x), Math.max(0, vw - w)),
    y: Math.min(Math.max(0, r.y), Math.max(0, vh - h)),
  };
}

export const useFloating = create<FloatingState>()(
  persist(
    (set) => ({
      panels: {},
      order: [],
      open: (id, initial) =>
        set((s) => {
          const saved = s.panels[id];
          const rect = fit(saved ?? initial);
          return {
            panels: { ...s.panels, [id]: { ...rect, open: true } },
            order: [...s.order.filter((x) => x !== id), id],
          };
        }),
      close: (id) =>
        set((s) =>
          s.panels[id] ? { panels: { ...s.panels, [id]: { ...s.panels[id], open: false } } } : s,
        ),
      setRect: (id, rect) =>
        set((s) =>
          s.panels[id] ? { panels: { ...s.panels, [id]: { ...s.panels[id], ...rect } } } : s,
        ),
      front: (id) =>
        set((s) =>
          s.order[s.order.length - 1] === id
            ? s
            : { order: [...s.order.filter((x) => x !== id), id] },
        ),
    }),
    {
      name: "photo-cat.floating",
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({ panels: s.panels, order: s.order }),
    },
  ),
);
