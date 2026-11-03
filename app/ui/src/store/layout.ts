// Stato dell'interfaccia persistito tra i riavvii: modulo attivo, pannelli, tema, lingua.

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { mirroredStorage } from "../lib/uiPrefs";
import type { ThemeName } from "../theme/tokens";
import type { Language } from "../i18n";
import type { PeopleSection } from "../lib/ipc";

export type ModuleId =
  | "library"
  | "sequence"
  | "catalogs"
  | "tags"
  | "people"
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

/** Destinazione per aprire il modulo Persone su una persona, un volto o una foto. */
export type PeopleTarget =
  { person: number } | { face: number } | { media: number; person?: number | null };

interface LayoutState {
  module: ModuleId;
  /** Sezione attiva del modulo Persone (Da rivedere / Persone / Ignorati). */
  peopleSection: PeopleSection;
  leftOpen: boolean;
  rightOpen: boolean;
  leftWidth: number;
  rightWidth: number;
  theme: ThemeName;
  language: Language;
  /** Filtro con cui aprire la Diagnostica (es. dal riepilogo di un job). */
  diagFilter: DiagFilter;
  /** Destinazione consumata da PeopleModule (non persistita). */
  peopleTarget: PeopleTarget | null;
  setModule: (m: ModuleId) => void;
  setPeopleSection: (s: PeopleSection) => void;
  toggleLeft: () => void;
  toggleRight: () => void;
  setLeftWidth: (w: number) => void;
  setRightWidth: (w: number) => void;
  setTheme: (t: ThemeName) => void;
  setLanguage: (l: Language) => void;
  openDiagnostics: (f: DiagFilter) => void;
  openPeople: (target: PeopleTarget) => void;
  clearPeopleTarget: () => void;
}

const clamp = (v: number, [min, max]: readonly [number, number]) => Math.min(max, Math.max(min, v));

type PersistedLayout = {
  module?: string;
  peopleSection?: PeopleSection;
  leftOpen?: boolean;
  rightOpen?: boolean;
  leftWidth?: number;
  rightWidth?: number;
  theme?: ThemeName;
  language?: Language;
};

/** Migra preferenze: la voce Identità non esiste più → modulo Persone, sezione Persone. */
export function migrateLayoutState(persisted: unknown): PersistedLayout {
  const s = (persisted ?? {}) as PersistedLayout;
  if (s.module === "identities") {
    return { ...s, module: "people", peopleSection: "people" };
  }
  return {
    ...s,
    peopleSection: s.peopleSection ?? "review",
  };
}

export const useLayout = create<LayoutState>()(
  persist(
    (set) => ({
      module: "library",
      peopleSection: "review",
      leftOpen: true,
      rightOpen: true,
      leftWidth: 240,
      rightWidth: 300,
      theme: "dark",
      language: "it",
      diagFilter: {},
      peopleTarget: null,
      setModule: (module) => set({ module }),
      setPeopleSection: (peopleSection) => set({ peopleSection }),
      toggleLeft: () => set((s) => ({ leftOpen: !s.leftOpen })),
      toggleRight: () => set((s) => ({ rightOpen: !s.rightOpen })),
      setLeftWidth: (w) => set({ leftWidth: clamp(w, PANEL_LIMITS.left) }),
      setRightWidth: (w) => set({ rightWidth: clamp(w, PANEL_LIMITS.right) }),
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
      openDiagnostics: (diagFilter) => set({ diagFilter, module: "diagnostics" }),
      openPeople: (peopleTarget) => set({ peopleTarget, module: "people" }),
      clearPeopleTarget: () => set({ peopleTarget: null }),
    }),
    {
      name: "photo-cat.layout",
      version: 1,
      migrate: migrateLayoutState,
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({
        module: s.module,
        peopleSection: s.peopleSection,
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
