// Modello di selezione comune (unified-selection, design D1): funzioni pure, senza React.
// Vale per qualunque raccolta (foto, volti, gruppi, persone, cataloghi) con id numerici o testuali.
import type { SelectionBase } from "../ipc";

/** Id di un elemento selezionabile: numerico (foto, volti, gruppi) o testuale (uid delle identità). */
export type Key = number | string;

/**
 * Selezione come insieme: elementi scelti uno per uno, oppure "tutti i risultati del filtro"
 * (Seleziona tutto) meno le eccezioni. Con 100.000 risultati non si carica mai l'elenco degli id:
 * `maxId` limita la selezione agli elementi presenti in quell'istante, `total` è il loro numero.
 * La forma per filtro ha senso solo con id numerici crescenti; le raccolte con id testuali
 * usano sempre gli id espliciti.
 */
export type Selection<F = unknown, Id extends Key = number> =
  | { mode: "ids"; ids: ReadonlySet<Id> }
  | {
      mode: "filter";
      filter: F;
      maxId: number;
      total: number;
      except: ReadonlySet<Id>;
    };

export const idsSelection = <F = unknown, Id extends Key = number>(
  ids: Iterable<Id> = [],
): Selection<F, Id> => ({
  mode: "ids",
  ids: new Set(ids),
});

/** "Seleziona tutto": l'insieme definito dal backend (`base`), senza eccezioni. */
export function selectAllOf<F, Id extends Key = number>(
  filter: F,
  base: SelectionBase,
): Selection<F, Id> {
  return { mode: "filter", filter, maxId: base.maxId, total: base.count, except: new Set<Id>() };
}

/** Inversione: gli id scelti diventano le eccezioni dell'insieme completo e viceversa. */
export function invertOf<F, Id extends Key = number>(
  s: Selection<F, Id>,
  filter: F,
  base: SelectionBase,
): Selection<F, Id> {
  return s.mode === "ids"
    ? { mode: "filter", filter, maxId: base.maxId, total: base.count, except: s.ids }
    : idsSelection<F, Id>(s.except);
}

/** Numero di elementi selezionati. */
export const selCount = <F, Id extends Key>(s: Selection<F, Id>) =>
  s.mode === "ids" ? s.ids.size : Math.max(0, s.total - s.except.size);

/** L'elemento è selezionato (O(1)). */
export const selHas = <F, Id extends Key>(s: Selection<F, Id>, id: Id) =>
  s.mode === "ids" ? s.ids.has(id) : (typeof id !== "number" || id <= s.maxId) && !s.except.has(id);

/** Id espliciti, o `null` per una selezione per filtro. */
export const selIds = <F, Id extends Key>(s: Selection<F, Id>): Id[] | null =>
  s.mode === "ids" ? [...s.ids] : null;

/** Aggiunge (`on`) o toglie elementi dalla selezione, in entrambe le modalità. */
export function selSet<F, Id extends Key>(
  s: Selection<F, Id>,
  ids: Iterable<Id>,
  on: boolean,
): Selection<F, Id> {
  if (s.mode === "ids") {
    const next = new Set(s.ids);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
    }
    return { mode: "ids", ids: next };
  }
  const except = new Set(s.except);
  for (const id of ids) {
    if (on) except.delete(id);
    else except.add(id);
  }
  return { ...s, except };
}

/** Clic su un elemento: singolo, Ctrl/Cmd (aggiungi/togli), Maiusc (intervallo dall'ancora). */
export function clickSelect<Id extends Key>(
  order: readonly Id[],
  current: readonly Id[],
  anchor: Id | null,
  id: Id,
  mods: { ctrl: boolean; shift: boolean },
): Id[] {
  if (mods.shift && anchor !== null) {
    const a = order.indexOf(anchor);
    const b = order.indexOf(id);
    if (a >= 0 && b >= 0) {
      const range = order.slice(Math.min(a, b), Math.max(a, b) + 1);
      return mods.ctrl ? Array.from(new Set([...current, ...range])) : range;
    }
  }
  if (mods.ctrl) return current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
  return [id];
}

/** Rettangolo nello spazio del contenuto, indipendente dai nodi virtualizzati. */
export function rectangleSelect<Id extends Key>(
  order: readonly Id[],
  box: { x: number; y: number; w: number; h: number },
  layout: { cols: number; cell: number; rowHeight: number; gap: number; pad: number },
  /** Posizione globale del primo elemento di `order` (griglia a finestra). */
  offset = 0,
): Id[] {
  const { cols, cell, rowHeight, gap, pad } = layout;
  return order.filter((_, k) => {
    const i = k + offset;
    const x = pad + (i % cols) * (cell + gap),
      y = pad + Math.floor(i / cols) * rowHeight;
    return (
      x < box.x + box.w && x + cell > box.x && y < box.y + box.h && y + rowHeight - gap > box.y
    );
  });
}

/**
 * Menu contestuale (spec "Menu contestuale sulla selezione", design D5): un clic destro su un
 * elemento non selezionato lo seleziona da solo; su un elemento selezionato la selezione resta.
 * Restituisce la selezione da usare (la stessa istanza se non cambia).
 */
export function selectForContextMenu<F, Id extends Key>(
  s: Selection<F, Id>,
  id: Id,
): Selection<F, Id> {
  return selHas(s, id) ? s : idsSelection<F, Id>([id]);
}
