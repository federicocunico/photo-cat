// Trasformazione condivisa dai pannelli di confronto (porting della logica di zoom di FujiView):
// un solo `Viewport` per tutti, così rotellina e trascinamento muovono ogni pannello insieme.

export type Viewport = { scale: number; x: number; y: number };

/** Immagine adattata al pannello, senza spostamento. */
export const FIT: Viewport = { scale: 1, x: 0, y: 0 };

export const MIN_SCALE = 1;
export const MAX_SCALE = 8;
/** Ingrandimento del doppio clic sull'immagine adattata. */
export const DOUBLE_CLICK_SCALE = 2;
const WHEEL_IN = 1.15;
const WHEEL_OUT = 0.87;

/** Zoom con la rotellina (verso l'alto ingrandisce); a scala 1 si torna alla vista adattata. */
export function zoomBy(view: Viewport, deltaY: number): Viewport {
  const factor = deltaY < 0 ? WHEEL_IN : WHEEL_OUT;
  const scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, view.scale * factor));
  return scale <= MIN_SCALE ? FIT : { ...view, scale };
}

/** Sposta l'immagine di (dx, dy) pixel; senza effetto se è adattata. */
export function panBy(view: Viewport, dx: number, dy: number): Viewport {
  return view.scale <= MIN_SCALE ? view : { ...view, x: view.x + dx, y: view.y + dy };
}

/** Doppio clic: adatta se è ingrandita, altrimenti ingrandisce. */
export function toggleZoom(view: Viewport): Viewport {
  return view.scale <= MIN_SCALE ? { scale: DOUBLE_CLICK_SCALE, x: 0, y: 0 } : FIT;
}

export function viewportTransform(view: Viewport): string {
  return `translate3d(${view.x}px, ${view.y}px, 0) scale(${view.scale})`;
}
