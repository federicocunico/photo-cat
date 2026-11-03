// Stato dell'interfaccia persistito tra i riavvii: modulo attivo, pannelli, tema, lingua.

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { mirroredStorage } from "../lib/uiPrefs";
import type { ThemePref } from "../theme/tokens";
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
  | "settings";

/**
 * Sezioni di Impostazioni raggiungibili direttamente (design D2, D9). Modelli AI e
 * Diagnostica non sono più voci della navigazione ma sezioni di Impostazioni.
 */
export type SettingsSection = "general" | "models" | "diagnostics";

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

/** Dove portare l'utente: un modulo, ed eventualmente una sezione o un elemento. */
export type Destination =
  | { module: ModuleId }
  | { module: "jobs"; target: number }
  | { module: "people"; target: PeopleTarget }
  | { module: "settings"; section: Exclude<SettingsSection, "diagnostics"> }
  | { module: "settings"; section: "diagnostics"; target?: DiagFilter };

interface LayoutState {
  module: ModuleId;
  /** Sezione attiva di Impostazioni. */
  settingsSection: SettingsSection;
  /** Sezione attiva del modulo Persone (Da rivedere / Persone / Ignorati). */
  peopleSection: PeopleSection;
  /** Navigazione dei moduli a sole icone (Ctrl+B). */
  navCompact: boolean;
  leftOpen: boolean;
  rightOpen: boolean;
  /** Sezioni richiuse dei pannelli laterali (es. "library.catalogs"). */
  collapsedSections: string[];
  leftWidth: number;
  rightWidth: number;
  theme: ThemePref;
  language: Language;
  /** Filtro con cui aprire la Diagnostica (es. dal riepilogo di un job). */
  diagFilter: DiagFilter;
  /** Destinazione consumata da PeopleModule (non persistita). */
  peopleTarget: PeopleTarget | null;
  /** Attività da mostrare all'apertura di Attività (non persistita). */
  jobFocus: number | null;
  /** Apre un modulo, una sezione di Impostazioni o un elemento (persona, filtro degli errori). */
  openModule: (to: Destination) => void;
  setSettingsSection: (s: SettingsSection) => void;
  setPeopleSection: (s: PeopleSection) => void;
  toggleNav: () => void;
  toggleLeft: () => void;
  toggleRight: () => void;
  /** Tab: nasconde entrambi i pannelli laterali, poi li ripristina com'erano. */
  toggleSidePanels: () => void;
  toggleSection: (id: string) => void;
  setLeftWidth: (w: number) => void;
  setRightWidth: (w: number) => void;
  setTheme: (t: ThemePref) => void;
  setLanguage: (l: Language) => void;
  clearPeopleTarget: () => void;
}

/** Pannelli com'erano prima di Tab, per ripristinarli al Tab successivo. */
let hiddenPanels: { leftOpen: boolean; rightOpen: boolean } | null = null;

const clamp = (v: number, [min, max]: readonly [number, number]) => Math.min(max, Math.max(min, v));

type PersistedLayout = {
  module?: string;
  settingsSection?: SettingsSection;
  peopleSection?: PeopleSection;
  navCompact?: boolean;
  leftOpen?: boolean;
  rightOpen?: boolean;
  collapsedSections?: string[];
  leftWidth?: number;
  rightWidth?: number;
  theme?: ThemePref;
  language?: Language;
};

/**
 * Migra le preferenze salvate:
 * - v0 → v1: la voce Identità non esiste più → modulo Persone, sezione Persone;
 * - v1 → v2: Modelli AI e Diagnostica diventano sezioni di Impostazioni.
 */
export function migrateLayoutState(persisted: unknown): PersistedLayout {
  const s = { ...((persisted ?? {}) as PersistedLayout) };
  if (s.module === "identities") Object.assign(s, { module: "people", peopleSection: "people" });
  if (s.module === "models" || s.module === "diagnostics")
    Object.assign(s, { module: "settings", settingsSection: s.module });
  return {
    ...s,
    peopleSection: s.peopleSection ?? "review",
    settingsSection: s.settingsSection ?? "general",
  };
}

/** Destinazione per un nome di modulo libero (preferenze vecchie, variabili di sviluppo). */
export function destinationFor(id: string): Destination | null {
  if (id === "models" || id === "diagnostics") return { module: "settings", section: id };
  if (id === "identities") return { module: "people" };
  const known: ModuleId[] = [
    "library",
    "sequence",
    "catalogs",
    "tags",
    "people",
    "import",
    "jobs",
    "organize",
    "settings",
  ];
  return known.includes(id as ModuleId) ? { module: id as ModuleId } : null;
}

export const useLayout = create<LayoutState>()(
  persist(
    (set) => ({
      module: "library",
      settingsSection: "general",
      peopleSection: "review",
      navCompact: false,
      leftOpen: true,
      rightOpen: true,
      collapsedSections: [],
      leftWidth: 240,
      rightWidth: 300,
      // Nuove installazioni: tema del sistema. Le preferenze salvate restano come sono.
      theme: "system",
      language: "it",
      diagFilter: {},
      peopleTarget: null,
      jobFocus: null,
      openModule: (to) => {
        const next: Partial<LayoutState> = { module: to.module };
        if ("section" in to) next.settingsSection = to.section;
        if ("target" in to) {
          if (to.module === "people") next.peopleTarget = to.target;
          else if (to.module === "jobs") next.jobFocus = to.target;
          else next.diagFilter = to.target ?? {};
        } else if ("section" in to && to.section === "diagnostics") next.diagFilter = {};
        set(next);
      },
      setSettingsSection: (settingsSection) => set({ settingsSection }),
      setPeopleSection: (peopleSection) => set({ peopleSection }),
      toggleNav: () => set((s) => ({ navCompact: !s.navCompact })),
      toggleLeft: () => set((s) => ({ leftOpen: !s.leftOpen })),
      toggleRight: () => set((s) => ({ rightOpen: !s.rightOpen })),
      toggleSidePanels: () =>
        set((s) => {
          if (s.leftOpen || s.rightOpen) {
            hiddenPanels = { leftOpen: s.leftOpen, rightOpen: s.rightOpen };
            return { leftOpen: false, rightOpen: false };
          }
          const restore = hiddenPanels ?? { leftOpen: true, rightOpen: true };
          hiddenPanels = null;
          return restore;
        }),
      toggleSection: (id) =>
        set((s) => ({
          collapsedSections: s.collapsedSections.includes(id)
            ? s.collapsedSections.filter((x) => x !== id)
            : [...s.collapsedSections, id],
        })),
      setLeftWidth: (w) => set({ leftWidth: clamp(w, PANEL_LIMITS.left) }),
      setRightWidth: (w) => set({ rightWidth: clamp(w, PANEL_LIMITS.right) }),
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
      clearPeopleTarget: () => set({ peopleTarget: null }),
    }),
    {
      name: "photo-cat.layout",
      version: 2,
      migrate: migrateLayoutState,
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({
        module: s.module,
        settingsSection: s.settingsSection,
        peopleSection: s.peopleSection,
        navCompact: s.navCompact,
        leftOpen: s.leftOpen,
        rightOpen: s.rightOpen,
        collapsedSections: s.collapsedSections,
        leftWidth: s.leftWidth,
        rightWidth: s.rightWidth,
        theme: s.theme,
        language: s.language,
      }),
    },
  ),
);
