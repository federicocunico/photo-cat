import { create } from "zustand";

/** Foglio delle scorciatoie aperto o chiuso (tasto "?", menu Aiuto). */
export const useShortcutSheet = create<{ open: boolean; setOpen: (open: boolean) => void }>(
  (set) => ({ open: false, setOpen: (open) => set({ open }) }),
);
