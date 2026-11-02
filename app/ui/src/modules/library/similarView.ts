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
