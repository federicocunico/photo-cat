// Ambito precompilato per la sezione Rianalisi (es. dall'azione "Rianalizza" della Libreria).
import { create } from "zustand";
import type { ReprocessScope } from "../../../lib/ipc";

type ReprocessPreset = {
  scope: ReprocessScope | null;
  /** Foto del bersaglio, per l'etichetta dell'ambito. */
  count: number | null;
  set: (p: { scope: ReprocessScope; count?: number | null }) => void;
  clear: () => void;
};

export const useReprocessPreset = create<ReprocessPreset>()((set) => ({
  scope: null,
  count: null,
  set: (p) => set({ scope: p.scope, count: p.count ?? null }),
  clear: () => set({ scope: null, count: null }),
}));
