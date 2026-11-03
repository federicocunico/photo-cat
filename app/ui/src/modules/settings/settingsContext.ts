// Contesto delle Impostazioni: valori in vigore, bozza da applicare e salvataggio
// immediato delle preferenze (vedi `settingsState.tsx`).
import { createContext, useContext } from "react";
import type { Settings, Taxonomy } from "../../lib/ipc";
import { thresholdsOf } from "./tuning";

export const SAVE_DELAY = 400;

export type FieldStatus = { state: "saving" | "saved" | "error"; message?: string };
export type Patch = (c: Settings) => Settings;

export type Ctx = {
  /** Configurazione in vigore (con le preferenze appena modificate). */
  saved: Settings;
  /** Bozza dei valori da applicare (soglie, dimensione dei volti, percorso dei dati). */
  draft: Settings;
  setDraft: (c: Settings) => void;
  savedTax: Taxonomy | undefined;
  /** Bozza della tassonomia (voci e soglie di gruppi e categorie). */
  tax: Taxonomy | null;
  setTax: (t: Taxonomy) => void;
  /** Salva subito una preferenza (dopo `SAVE_DELAY`). */
  autosave: (field: string, patch: Patch, after?: (next: Settings, prev: Settings) => void) => void;
  status: Record<string, FieldStatus>;
  /** Campo da evidenziare (dalla ricerca). */
  highlight: string | null;
  setHighlight: (id: string | null) => void;
};

export const SettingsCtx = createContext<Ctx | null>(null);

export function useSettings(): Ctx {
  const ctx = useContext(SettingsCtx);
  if (!ctx) throw new Error("useSettings fuori da SettingsProvider");
  return ctx;
}

/** Le parti della configurazione che si applicano con "Applica". */
export function applyFields(c: Settings) {
  return {
    thresholds: thresholdsOf(c),
    face_min_size_px: c.ai.face_min_size_px ?? null,
    confidence_threshold: c.ai.confidence_threshold ?? null,
    data_dir: c.data_dir ?? null,
  };
}

/** Configurazione in vigore con i valori da applicare della bozza. */
export function withDraft(saved: Settings, draft: Settings): Settings {
  return {
    ...saved,
    ai: {
      ...saved.ai,
      thresholds: thresholdsOf(draft),
      face_min_size_px: draft.ai.face_min_size_px,
      confidence_threshold: draft.ai.confidence_threshold,
    },
    data_dir: draft.data_dir ?? null,
  };
}
