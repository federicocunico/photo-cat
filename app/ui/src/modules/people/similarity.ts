// Somiglianza tra volti in parole (ui-usability-audit, D4): fuori da Impostazioni › Avanzate e
// dalla Diagnostica i punteggi tra 0 e 1 non si mostrano come numeri.

/**
 * Sopra questa somiglianza coseno (SFace) la somiglianza è "alta": è la soglia d'accettazione
 * di PhotoPrism per lo stesso modello. Provvisoria: la change sull'aggregazione delle identità
 * la ricalibra sui dati.
 */
export const HIGH_SIMILARITY = 0.55;

export type SimilarityLevel = "high" | "medium";

export const similarityLevel = (score: number): SimilarityLevel =>
  score >= HIGH_SIMILARITY ? "high" : "medium";
