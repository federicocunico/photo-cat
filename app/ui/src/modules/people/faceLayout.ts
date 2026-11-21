// Disposizione della griglia dei volti con intestazioni per anno (ordinamenti per data).
// Le righe coprono tutti i volti dell'ambito (conteggi del backend), anche quelli
// delle pagine non ancora caricate: `from` è la posizione nell'elenco ordinato.
import type { FaceYear } from "../../lib/ipc";

export type FaceRow =
  | { kind: "head"; year: number | null; count: number }
  | { kind: "faces"; from: number; to: number };

/** Anni nell'ordine della griglia; "Senza data" resta sempre in fondo. */
export function orderYears(years: FaceYear[], sort: "captured_asc" | "captured_desc"): FaceYear[] {
  const dated = years.filter((y) => y.year != null);
  dated.sort((a, b) => (sort === "captured_asc" ? a.year! - b.year! : b.year! - a.year!));
  return [...dated, ...years.filter((y) => y.year == null)];
}

export function buildFaceRows(years: FaceYear[], cols: number): FaceRow[] {
  const rows: FaceRow[] = [];
  let at = 0;
  for (const y of years) {
    if (y.count <= 0) continue;
    rows.push({ kind: "head", year: y.year, count: y.count });
    for (let i = 0; i < y.count; i += cols)
      rows.push({ kind: "faces", from: at + i, to: at + Math.min(y.count, i + cols) });
    at += y.count;
  }
  return rows;
}

/** Selezione con rettangolo su una griglia con intestazioni (righe di altezza diversa). */
export function rectangleSelectRows(
  order: number[],
  box: { x: number; y: number; w: number; h: number },
  rows: FaceRow[],
  g: { cell: number; rowHeight: number; headHeight: number; gap: number; pad: number },
): number[] {
  const out: number[] = [];
  let top = g.pad;
  for (const r of rows) {
    if (r.kind === "head") {
      top += g.headHeight;
      continue;
    }
    const y = top;
    top += g.rowHeight;
    if (!(y < box.y + box.h && y + g.rowHeight - g.gap > box.y)) continue;
    for (let i = r.from; i < r.to && i < order.length; i++) {
      const x = g.pad + (i - r.from) * (g.cell + g.gap);
      if (x < box.x + box.w && x + g.cell > box.x) out.push(order[i]);
    }
  }
  return out;
}
