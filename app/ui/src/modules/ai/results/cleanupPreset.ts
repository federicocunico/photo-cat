// Parametri precompilati per la sezione Pulizia (es. dall'istogramma dei Risultati).
import { create } from "zustand";

export type CleanupPresetValues = {
  grp: string | null;
  value: string | null;
  maxScore: number | null;
  scopeCatalogId: number | null;
};

type CleanupPreset = CleanupPresetValues & {
  set: (p: Partial<CleanupPresetValues>) => void;
  /** Alias di `set`. */
  setPreset: (p: Partial<CleanupPresetValues>) => void;
  clear: () => void;
};

const empty: CleanupPresetValues = { grp: null, value: null, maxScore: null, scopeCatalogId: null };

export const useCleanupPreset = create<CleanupPreset>()((set) => ({
  ...empty,
  set: (p) => set({ ...empty, ...p }),
  setPreset: (p) => set({ ...empty, ...p }),
  clear: () => set(empty),
}));
