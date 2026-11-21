// Scelte da esportare (porting di FujiView `export.ts`). Nella revisione i file non hanno un
// percorso lato interfaccia: la copia la fa il backend a partire da scatto e preset.
// Regola di photo-cat: si esportano solo le varianti JPEG (`exportable`); l'anteprima RAW, i TIFF
// e i RAF non entrano mai nella destinazione.
import type { ReviewShotView, ShotDecision } from "../../lib/ipc";

export type ExportItem = {
  shotKey: string;
  preset: string;
  /** Nome del file della variante (prima della rinomina con il modello). */
  outputName: string;
  rating: number;
};

type Decisions = Record<string, ShotDecision | undefined>;

function winnerVariant(shot: ReviewShotView, decision: ShotDecision | undefined) {
  if (!decision?.winnerPreset) return undefined;
  return shot.variants.find((v) => v.preset === decision.winnerPreset);
}

/** Varianti migliori da esportare, con valutazione almeno `minRating` (0 = tutte le scelte). */
export function buildExportItems(
  shots: readonly ReviewShotView[],
  decisions: Decisions,
  minRating: number,
): ExportItem[] {
  const items: ExportItem[] = [];
  for (const shot of shots) {
    const decision = decisions[shot.key];
    const variant = winnerVariant(shot, decision);
    if (!decision || !variant?.exportable || decision.rating < minRating) continue;
    items.push({
      shotKey: shot.key,
      preset: variant.preset,
      outputName: variant.fileName,
      rating: decision.rating,
    });
  }
  return items;
}

/** Scelte che non si possono esportare perché la variante non è un JPEG (o non esiste più). */
export function countNotExportable(
  shots: readonly ReviewShotView[],
  decisions: Decisions,
  minRating = 0,
): number {
  let count = 0;
  for (const shot of shots) {
    const decision = decisions[shot.key];
    if (!decision?.winnerPreset || decision.rating < minRating) continue;
    if (!winnerVariant(shot, decision)?.exportable) count += 1;
  }
  return count;
}

/** Numero di varianti esportabili per valutazione esatta (indici 0..5). */
export function countByRating(shots: readonly ReviewShotView[], decisions: Decisions): number[] {
  const counts = [0, 0, 0, 0, 0, 0];
  for (const item of buildExportItems(shots, decisions, 0)) {
    counts[Math.min(5, Math.max(0, item.rating))] += 1;
  }
  return counts;
}

/** Varianti con valutazione >= minRating, dai conteggi per valutazione. */
export function countAtLeast(counts: readonly number[], minRating: number): number {
  return counts.slice(Math.max(0, minRating)).reduce((sum, value) => sum + value, 0);
}
