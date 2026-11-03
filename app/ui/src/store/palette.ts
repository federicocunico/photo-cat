import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { mirroredStorage } from "../lib/uiPrefs";

/** Quanti comandi recenti la palette ricorda. */
export const RECENT_LIMIT = 5;

type PaletteState = {
  open: boolean;
  /** Testo iniziale (es. digitato nel campo della barra superiore). */
  initial: string;
  /** Id dei comandi usati dalla palette, dal più recente. */
  recent: string[];
  openPalette: (initial?: string) => void;
  close: () => void;
  remember: (id: string) => void;
};

export const usePalette = create<PaletteState>()(
  persist(
    (set) => ({
      open: false,
      initial: "",
      recent: [],
      openPalette: (initial = "") => set({ open: true, initial }),
      close: () => set({ open: false, initial: "" }),
      remember: (id) =>
        set((s) => ({ recent: [id, ...s.recent.filter((x) => x !== id)].slice(0, RECENT_LIMIT) })),
    }),
    {
      name: "photo-cat.palette",
      version: 1,
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({ recent: s.recent }),
    },
  ),
);

/** Selettore della barra delle azioni da aprire (es. "Aggiungi tag…" dalla palette). */
export const useSelectionPicker = create<{ open: "tag" | null; set: (p: "tag" | null) => void }>(
  (set) => ({ open: null, set: (open) => set({ open }) }),
);
