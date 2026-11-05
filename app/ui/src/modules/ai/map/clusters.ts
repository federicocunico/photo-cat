// Logica pura dello strato dei gruppi della mappa (photo-map D3, D5): dimensioni e hit-test.
// Un gruppo è esattamente una cella del backend: i cerchi non si fondono mai, così il clic
// apre la Libreria con la stessa area e lo stesso conteggio. Niente canvas né React.
import type { Cell } from "../../../bindings";
import { worldPx } from "../../../components/map/projection";

export const MIN_RADIUS = 6;
export const MAX_RADIUS = 30;
/** Celle per lato a zoom di griglia 0 (come `CELLS_PER_TILE` del backend). */
const CELLS_PER_TILE = 8;
/** Lato minimo di una cella sullo schermo, in pixel. */
export const MIN_CELL_PX = 24;
/** Livello massimo di griglia supportato dal backend. */
export const MAX_GRID_ZOOM = 24;

/** Livello di griglia da chiedere al backend: segue la vista, celle di ~24–48 px. */
export function gridZoom(viewZoom: number): number {
  const z = Math.floor(Math.log2((worldPx(Math.max(viewZoom, 1)) / CELLS_PER_TILE / MIN_CELL_PX) * (1 + 1e-9)));
  return Math.max(0, Math.min(MAX_GRID_ZOOM, z));
}

/** Lato in pixel di una cella della griglia `zoom` alla vista `viewZoom`. */
export function cellPx(viewZoom: number, zoom: number): number {
  return worldPx(viewZoom) / (CELLS_PER_TILE * 2 ** zoom);
}

/** Raggio proporzionale a sqrt(count), tra [MIN_RADIUS, MAX_RADIUS]. */
export function radiusFor(count: number): number {
  return Math.min(MAX_RADIUS, Math.max(MIN_RADIUS, 4 + 2.2 * Math.sqrt(Math.max(count, 0))));
}

/** Un gruppo disegnato: una cella. */
export interface Cluster {
  x: number;
  y: number;
  r: number;
  count: number;
  approxCount: number;
  place: string | null;
  sampleIds: number[];
  cell: Cell;
}

/** Il gruppo è fatto soprattutto di foto con posizione approssimata. */
export const isApproxCluster = (c: Pick<Cluster, "count" | "approxCount">) => c.approxCount * 2 > c.count;

type Project = (lon: number, lat: number) => [number, number];

/**
 * Posiziona le celle in pixel. Il raggio è limitato a 0,45 volte il lato della cella
 * (`cellSize`, px), così i cerchi di celle vicine non si toccano. Ordinato dal più grande
 * al più piccolo: i piccoli restano disegnati sopra.
 */
export function layoutClusters(cells: Cell[], project: Project, cellSize = Infinity): Cluster[] {
  const out: Cluster[] = [];
  for (const cell of cells) {
    if (cell.lat === null || cell.lon === null) continue;
    const [x, y] = project(cell.lon, cell.lat);
    out.push({
      x,
      y,
      r: Math.min(radiusFor(cell.count), 0.45 * cellSize),
      count: cell.count,
      approxCount: cell.approxCount,
      place: cell.place,
      sampleIds: [...cell.sampleIds],
      cell,
    });
  }
  return out.sort((a, b) => b.count - a.count);
}

/** Gruppo sotto il punto (il più piccolo se ce ne sono più d'uno), o null. */
export function hitTest(clusters: Cluster[], x: number, y: number): Cluster | null {
  let best: Cluster | null = null;
  for (const c of clusters) {
    if (Math.hypot(c.x - x, c.y - y) <= c.r && (!best || c.r < best.r)) best = c;
  }
  return best;
}

/** Etichetta del gruppo: luogo prevalente o coordinate. */
export function clusterLabel(cluster: Pick<Cluster, "place" | "cell">): string {
  if (cluster.place) return cluster.place;
  const c = cluster.cell;
  return c.lat !== null && c.lon !== null ? `${c.lat.toFixed(2)}, ${c.lon.toFixed(2)}` : "";
}

/** Etichetta di una cella (stessa regola di `clusterLabel`). */
export const cellLabel = (cell: Cell): string => clusterLabel({ place: cell.place, cell });

/** Disegna i gruppi: pieni, oppure tratteggiati e più chiari se quasi tutti approssimati. */
export function drawClusters(
  ctx: CanvasRenderingContext2D,
  clusters: Cluster[],
  colors: { accent: string; text: string; ink?: string },
  hovered: Cluster | null,
) {
  // Sui gruppi approssimati (riempimento al 30%) il testo sta sullo sfondo della mappa: usa il
  // colore del testo del tema (`ink`), non quello pensato per il pieno dell'accento.
  for (const c of clusters) {
    ctx.beginPath();
    ctx.arc(c.x, c.y, c.r, 0, Math.PI * 2);
    ctx.lineWidth = hovered && c.cell === hovered.cell ? 3 : 1.5;
    ctx.strokeStyle = colors.accent;
    if (isApproxCluster(c)) {
      ctx.globalAlpha = 0.3;
      ctx.fillStyle = colors.accent;
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.setLineDash([4, 3]);
      ctx.stroke();
      ctx.setLineDash([]);
    } else {
      ctx.globalAlpha = 0.75;
      ctx.fillStyle = colors.accent;
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.stroke();
    }
    if (c.r >= 12) {
      ctx.fillStyle = isApproxCluster(c) ? (colors.ink ?? colors.text) : colors.text;
      ctx.font = "600 11px system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(c.count > 999 ? `${Math.round(c.count / 100) / 10}k` : String(c.count), c.x, c.y);
    }
  }
}
