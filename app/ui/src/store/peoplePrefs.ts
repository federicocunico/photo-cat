// Preferenze del modulo Persone ricordate dopo il riavvio (`people.faceSort`).
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import type { FaceSort } from "../bindings";
import { mirroredStorage } from "../lib/uiPrefs";

export const FACE_SORTS: FaceSort[] = ["quality_desc", "captured_asc", "captured_desc"];

type PeoplePrefs = {
  /** Ordinamento dei volti nella griglia: qualità (predefinito) o data di scatto. */
  faceSort: FaceSort;
  setFaceSort: (s: FaceSort) => void;
};

export const usePeoplePrefs = create<PeoplePrefs>()(
  persist(
    (set) => ({
      faceSort: "quality_desc",
      setFaceSort: (faceSort) => set({ faceSort }),
    }),
    {
      name: "photo-cat.people",
      storage: createJSONStorage(() => mirroredStorage),
      partialize: (s) => ({ faceSort: s.faceSort }),
      merge: (persisted, current) => {
        const p = (persisted ?? {}) as { faceSort?: FaceSort };
        return {
          ...current,
          faceSort: p.faceSort && FACE_SORTS.includes(p.faceSort) ? p.faceSort : current.faceSort,
        };
      },
    },
  ),
);
