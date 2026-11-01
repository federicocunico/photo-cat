// Stato dell'interfaccia persistito tra i riavvii: modulo attivo, pannelli, tema, lingua.

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { ThemeName } from "../theme/tokens";
import type { Language } from "../i18n";

export type ModuleId =
  | "library"
  | "sequence"
  | "catalogs"
  | "tags"
  | "people"
  | "identities"
  | "import"
  | "jobs"
  | "organize"
  | "models"
  | "diagnostics"
  | "settings";

export const PANEL_LIMITS = { left: [200, 420], right: [260, 480] } as const;

export interface DiagFilter {
  level?: string;
  phase?: string;
  jobId?: number;
  text?: string;
}

interface LayoutState {
  module: ModuleId;
  leftOpen: boolean;
  rightOpen: boolean;
  leftWidth: number;
  rightWidth: number;
  theme: ThemeName;
  language: Language;
  /** Filtro con cui aprire la Diagnostica (es. dal riepilogo di un job). */
  diagFilter: DiagFilter;
  setModule: (m: ModuleId) => void;
  toggleLeft: () => void;
  toggleRight: () => void;
  setLeftWidth: (w: number) => void;
  setRightWidth: (w: number) => void;
  setTheme: (t: ThemeName) => void;
  setLanguage: (l: Language) => void;
  openDiagnostics: (f: DiagFilter) => void;
}

const clamp = (v: number, [min, max]: readonly [number, number]) => Math.min(max, Math.max(min, v));

export const useLayout = create<LayoutState>()(
  persist(
    (set) => ({
      module: "library",
      leftOpen: true,
      rightOpen: true,
      leftWidth: 240,
      rightWidth: 300,
      theme: "dark",
      language: "it",
      diagFilter: {},
      setModule: (module) => set({ module }),
      toggleLeft: () => set((s) => ({ leftOpen: !s.leftOpen })),
      toggleRight: () => set((s) => ({ rightOpen: !s.rightOpen })),
      setLeftWidth: (w) => set({ leftWidth: clamp(w, PANEL_LIMITS.left) }),
      setRightWidth: (w) => set({ rightWidth: clamp(w, PANEL_LIMITS.right) }),
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
      openDiagnostics: (diagFilter) => set({ diagFilter, module: "diagnostics" }),
    }),
    {
      name: "photo-cat.layout",
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({
        module: s.module,
        leftOpen: s.leftOpen,
        rightOpen: s.rightOpen,
        leftWidth: s.leftWidth,
        rightWidth: s.rightWidth,
        theme: s.theme,
        language: s.language,
      }),
    },
  ),
);
