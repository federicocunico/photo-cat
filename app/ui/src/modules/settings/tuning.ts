// Soglie AI (design D9): valori predefiniti, verso di ogni soglia e campo della scala.
import type { AiThresholds, Settings } from "../../lib/ipc";

/** Valori predefiniti dell'applicazione (pc-core `AiThresholds`, tassonomia). */
export const DEFAULTS = {
  person_detection: 0.4,
  face_detection: 0.8,
  face_min_sharpness: 0.35,
  face_similarity: 0.363,
  similar_photos: 0.8,
  face_min_size_px: 40,
  group: 0.25,
};

export type ThresholdKey = keyof AiThresholds;
export type FullThresholds = Required<{ [K in ThresholdKey]: number }>;

/**
 * Verso di ogni soglia: `min` = è un minimo (confidenza, nitidezza, somiglianza), quindi
 * un valore più alto dà meno risultati ma più precisi; `max` = è una distanza massima,
 * quindi vale il contrario. Oggi tutte le soglie sono minimi: la tabella evita di
 * confondere confidenza e distanza se se ne aggiunge una.
 */
export type Direction = "min" | "max";

export const SCALE: Record<
  ThresholdKey | "group",
  { direction: Direction; lo: number; hi: number }
> = {
  person_detection: { direction: "min", lo: 0.1, hi: 0.95 },
  face_detection: { direction: "min", lo: 0.3, hi: 0.99 },
  face_min_sharpness: { direction: "min", lo: 0, hi: 0.95 },
  face_similarity: { direction: "min", lo: 0.15, hi: 0.7 },
  similar_photos: { direction: "min", lo: 0.5, hi: 0.99 },
  group: { direction: "min", lo: 0.05, hi: 0.8 },
};

/** Posizione sulla scala "Più risultati ↔ Più precisi", in percentuale. */
export function precision(key: keyof typeof SCALE, value: number): number {
  const v = Math.min(1, Math.max(0, value));
  return Math.round((SCALE[key].direction === "min" ? v : 1 - v) * 100);
}

/** Valore della soglia per una posizione della scala (inverso di `precision`). */
export function fromPrecision(key: keyof typeof SCALE, percent: number): number {
  const p = Math.min(100, Math.max(0, percent)) / 100;
  const v = SCALE[key].direction === "min" ? p : 1 - p;
  return Math.round(v * 1000) / 1000;
}

export function thresholdsOf(cfg: Settings): FullThresholds {
  const t = cfg.ai.thresholds ?? {};
  return {
    person_detection: t.person_detection ?? DEFAULTS.person_detection,
    face_detection: t.face_detection ?? DEFAULTS.face_detection,
    face_min_sharpness: t.face_min_sharpness ?? DEFAULTS.face_min_sharpness,
    face_similarity: t.face_similarity ?? DEFAULTS.face_similarity,
    similar_photos: t.similar_photos ?? DEFAULTS.similar_photos,
  };
}

/** Dimensione minima dei volti: tre livelli bastano (il valore esatto è in Avanzate). */
export const FACE_SIZES = { small: 24, normal: 40, large: 80 } as const;
