// Lato delle anteprime richieste a `review://` per un pannello.
//
// Il precaricamento del backend produce anteprime a 2048 px (`PREFETCH_EDGE`): finché il pannello
// non ha bisogno di più pixel si chiede proprio quel lato, così lo scatto precaricato compare
// subito dalla cache. Oltre si passa ai lati 4096 e 8192 (pannelli grandi su schermi densi, o zoom).

/** Lati richiesti, dal più piccolo; il primo è quello del precaricamento. */
export const PREVIEW_EDGES = [2048, 4096, 8192] as const;

/** Lato dell'anteprima per un pannello di `width` × `height` px CSS con zoom `scale`. */
export function previewEdge(width: number, height: number, ratio: number, scale: number): number {
  const need = Math.max(width, height) * Math.max(1, ratio) * Math.max(1, scale);
  return PREVIEW_EDGES.find((edge) => need <= edge) ?? PREVIEW_EDGES[PREVIEW_EDGES.length - 1];
}
