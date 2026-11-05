import type { AiSection } from "../../store/layout";

/**
 * Mostra la sezione Mappa nella sotto-navigazione (photo-map).
 */
export const SHOW_MAP = true;

/** Sezioni mostrate nella sotto-navigazione, nell'ordine. */
export const AI_SECTIONS: AiSection[] = [
  "overview",
  "reprocess",
  "results",
  "cleanup",
  "similar",
  ...(SHOW_MAP ? (["map"] as AiSection[]) : []),
];
