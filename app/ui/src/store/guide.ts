// Guida nell'app (in-app-guidance, design D8): "Primi passi", suggerimenti al primo uso,
// introduzione da rivedere. Persistito in `photo-cat.guide`, copiato in `ui-prefs.json`.
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { mirroredStorage } from "../lib/uiPrefs";

export type GuideStep = "import" | "models" | "person" | "catalog" | "tags";
export const GUIDE_STEPS: GuideStep[] = ["import", "models", "person", "catalog", "tags"];

/** Suggerimenti al primo uso; `moved` è per chi aggiorna (Modelli AI e Diagnostica). */
export type TipId = "people" | "catalogs" | "tags" | "sequence" | "moved";

/** Sezioni della guida (Aiuto › Guida e "Scopri di più"). */
export type GuideSection =
  "library" | "people" | "catalogs" | "tags" | "sequence" | "import" | "settings";

type GuideState = {
  done: Partial<Record<GuideStep, true>>;
  /** Lista "Primi passi" nascosta dall'utente (si riapre dal menu Aiuto). */
  hidden: boolean;
  /** Suggerimenti già mostrati: non ricompaiono. */
  seen: TipId[];
  /** Suggerimento visibile ora: al massimo uno alla volta. */
  active: TipId | null;
  /** Mostrati in questa sessione e non ancora chiusi: tornano se il modulo si riapre. */
  session: TipId[];
  /** Introduzione riaperta dal menu Aiuto. */
  intro: boolean;
  /** Sezione della guida aperta, o `null`. */
  guide: GuideSection | null;
  /** "Segnala un problema": Diagnostica mette in evidenza l'esportazione. */
  report: boolean;
  complete: (step: GuideStep) => void;
  hide: () => void;
  show: () => void;
  /** Mostra il suggerimento se non è già stato visto e non ce n'è un altro aperto. */
  claim: (tip: TipId) => boolean;
  release: (tip: TipId) => void;
  /** "Ho capito" (o Esc): il suggerimento non ricompare. */
  dismiss: (tip: TipId) => void;
  markSeen: (tip: TipId) => void;
  resetTips: () => void;
  setIntro: (open: boolean) => void;
  openGuide: (section: GuideSection | null) => void;
  setReport: (on: boolean) => void;
};

export const useGuide = create<GuideState>()(
  persist(
    (set, get) => ({
      done: {},
      hidden: false,
      seen: [],
      active: null,
      session: [],
      intro: false,
      guide: null,
      report: false,
      complete: (step) => {
        if (!get().done[step]) set((s) => ({ done: { ...s.done, [step]: true } }));
      },
      hide: () => set({ hidden: true }),
      show: () => set({ hidden: false }),
      claim: (tip) => {
        const s = get();
        if (s.active === tip) return true;
        if (s.active) return false;
        if (s.session.includes(tip)) {
          set({ active: tip });
          return true;
        }
        if (s.seen.includes(tip)) return false;
        // Segnato come visto appena compare: dopo un riavvio non si ripresenta.
        set({ active: tip, seen: [...s.seen, tip], session: [...s.session, tip] });
        return true;
      },
      release: (tip) => {
        if (get().active === tip) set({ active: null });
      },
      dismiss: (tip) =>
        set((s) => ({
          active: s.active === tip ? null : s.active,
          session: s.session.filter((x) => x !== tip),
        })),
      markSeen: (tip) => {
        if (!get().seen.includes(tip)) set((s) => ({ seen: [...s.seen, tip] }));
      },
      resetTips: () => set({ seen: [], session: [], active: null }),
      setIntro: (intro) => set({ intro }),
      openGuide: (guide) => set({ guide }),
      setReport: (report) => set({ report }),
    }),
    {
      name: "photo-cat.guide",
      version: 1,
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({ done: s.done, hidden: s.hidden, seen: s.seen }),
    },
  ),
);

/** Passi completati, nell'ordine della lista. */
export const doneCount = (done: GuideState["done"]) => GUIDE_STEPS.filter((s) => done[s]).length;
