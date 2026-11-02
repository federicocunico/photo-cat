// Griglia raggruppata: un gruppo aperto inserisce le sue altre foto subito dopo l'elemento che
// lo rappresenta. Qui si traducono le posizioni mostrate in posizioni della lista di pagine.

import type { GridItem } from "../../lib/ipc";

/** Gruppo aperto: posizione dell'elemento rappresentante e foto inserite dopo di lui. */
export interface Expansion {
  group: number;
  /** Posizione (nella lista delle pagine) dell'elemento che rappresenta il gruppo. */
  base: number;
  /** Altre foto del gruppo presenti nei risultati (esclusa la rappresentante). */
  members: GridItem[];
}

export const byBase = (e: Expansion[]) => [...e].sort((a, b) => a.base - b.base);

/** Elementi mostrati: quelli delle pagine più le foto dei gruppi aperti. */
export const displayTotal = (base: number, expansions: Expansion[]) =>
  base + expansions.reduce((n, e) => n + e.members.length, 0);

export type Slot = { base: number } | { member: GridItem; expansion: Expansion; index: number };

/** Cosa c'è alla posizione mostrata `d` (`expansions` ordinate per `base`). */
export function locate(d: number, expansions: Expansion[]): Slot {
  let extra = 0;
  for (const e of expansions) {
    const at = e.base + extra;
    if (d <= at) break;
    if (d <= at + e.members.length) {
      const index = d - at - 1;
      return { member: e.members[index], expansion: e, index };
    }
    extra += e.members.length;
  }
  return { base: d - extra };
}

/** Posizione nelle pagine corrispondente a quella mostrata (una foto aperta conta come il suo gruppo). */
export function toBase(d: number, expansions: Expansion[]): number {
  const s = locate(d, expansions);
  return "base" in s ? s.base : s.expansion.base;
}

/** Elemento con i dati del gruppo di foto simili (griglia della Libreria o della Sequenza). */
interface Groupable {
  id: number;
  similarGroup: number | null;
  similarCover: boolean;
}

/**
 * Raggruppamento sugli elementi già caricati (Sequenza): ogni gruppo chiuso diventa un solo
 * elemento, la principale se presente o altrimenti la prima foto, con il numero di foto
 * presenti; i gruppi in `open` restano interi.
 */
export function collapseLoaded<T extends Groupable>(
  items: T[],
  open: ReadonlySet<number>,
): Array<{ item: T; count: number | null }> {
  const counts = new Map<number, number>();
  const rep = new Map<number, T>();
  for (const it of items) {
    if (it.similarGroup == null) continue;
    counts.set(it.similarGroup, (counts.get(it.similarGroup) ?? 0) + 1);
    const cur = rep.get(it.similarGroup);
    if (!cur || (it.similarCover && !cur.similarCover)) rep.set(it.similarGroup, it);
  }
  const out: Array<{ item: T; count: number | null }> = [];
  for (const it of items) {
    const g = it.similarGroup;
    const n = g == null ? 1 : (counts.get(g) ?? 1);
    if (g == null || n < 2) out.push({ item: it, count: null });
    else if (open.has(g)) out.push({ item: it, count: rep.get(g) === it ? n : null });
    else if (rep.get(g) === it) out.push({ item: it, count: n });
  }
  return out;
}
