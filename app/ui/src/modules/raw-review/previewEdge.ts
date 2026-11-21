// Lato delle anteprime richieste a `review://` per un pannello.
//
// Il precaricamento del backend produce anteprime a 2048 px (`PREFETCH_EDGE`): finché il pannello
// non ha bisogno di più pixel si chiede proprio quel lato, così lo scatto precaricato compare
// subito dalla cache. Oltre si passa ai lati 4096 e 8192 (pannelli grandi su schermi densi, o zoom).

/** Lati richiesti, dal più piccolo; il primo è quello del precaricamento. */
export const PREVIEW_EDGES = [2048, 4096, 8192] as const;

/**
 * Lato massimo per numero di pannelli: ogni `<img>` decodificata a 8192 px occupa circa 180 MB
 * in memoria, quindi con molti pannelli ci si ferma prima (9 × 4096 px ≈ 400 MB).
 */
export function maxEdgeFor(panelCount: number): number {
  return panelCount <= 2 ? 8192 : 4096;
}

/** Lato dell'anteprima per un pannello di `width` × `height` px CSS con zoom `scale`. */
export function previewEdge(
  width: number,
  height: number,
  ratio: number,
  scale: number,
  maxEdge: number = PREVIEW_EDGES[PREVIEW_EDGES.length - 1],
): number {
  const need = Math.max(width, height) * Math.max(1, ratio) * Math.max(1, scale);
  const edge = PREVIEW_EDGES.find((e) => need <= e) ?? PREVIEW_EDGES[PREVIEW_EDGES.length - 1];
  return Math.max(PREVIEW_EDGES[0], Math.min(edge, maxEdge));
}
