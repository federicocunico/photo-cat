// Pannello a fuoco per la tastiera (Invio sceglie, le frecce lo spostano): logica pura.
import { panelGrid } from "./project";

export type Arrow = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown";

/**
 * Pannello a fuoco: quello scelto dall'utente, altrimenti la variante migliore dello scatto,
 * altrimenti il primo (come in FujiView). Sempre dentro i pannelli visibili.
 */
export function focusedPanel(
  panels: readonly string[],
  chosen: number | null,
  winnerPreset: string | null,
): number {
  if (panels.length === 0) return 0;
  const base = chosen ?? (winnerPreset ? panels.indexOf(winnerPreset) : -1);
  return Math.min(panels.length - 1, Math.max(0, base));
}

/** Posizione dopo una freccia sulla griglia dei pannelli; resta ferma ai bordi. */
export function moveFocus(current: number, count: number, key: Arrow): number {
  const { columns } = panelGrid(count);
  if (key === "ArrowLeft" && current % columns > 0) return current - 1;
  if (key === "ArrowRight" && current % columns < columns - 1 && current + 1 < count)
    return current + 1;
  if (key === "ArrowUp" && current - columns >= 0) return current - columns;
  if (key === "ArrowDown" && current + columns < count) return current + columns;
  return current;
}
