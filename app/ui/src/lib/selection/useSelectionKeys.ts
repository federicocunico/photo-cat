// Tasti della selezione (unified-selection, design D2): un'unica tabella "tasto × modificatore"
// per griglie ed elenchi, come in un gestore di file.
// - frecce, Home/Fine, Pagina su/giù: spostano il focus e selezionano solo l'elemento raggiunto;
// - con Maiusc estendono dall'ancora (con Ctrl+Maiusc aggiungono l'intervallo);
// - con Ctrl spostano solo il focus;
// - Ctrl+Spazio aggiunge o toglie l'elemento con il focus;
// - Ctrl+A seleziona tutto, Ctrl+Maiusc+I inverte, Esc svuota.
// Lavora per posizione (indice), così vale anche per raccolte virtualizzate o a finestra.
import { useRef, type KeyboardEvent } from "react";
import { isTyping } from "../keys";
import type { Key } from "./model";

export interface SelectionKeysOptions<Id extends Key> {
  /** Griglia (frecce su righe e colonne) o elenco (solo su e giù). */
  layout: "grid" | "list";
  /** Colonne della griglia (1 per un elenco). */
  columns?: number;
  /** Elementi per pagina (Pagina su/giù). */
  pageSize: number;
  /** Numero totale di elementi. */
  count: number;
  /** Posizione dell'elemento con il focus, -1 se nessuno. */
  cursor: number;
  /** Id dell'elemento in quella posizione, se caricato. */
  idAt: (index: number) => Id | undefined;
  /** Porta il focus sulla posizione e la rende visibile. */
  moveTo: (index: number) => void;
  /** Gesto di selezione su un elemento (stessa semantica del clic). */
  select: (id: Id, mods: { ctrl: boolean; shift: boolean }) => void;
  /**
   * Senza ancora, Maiusc+freccia parte dall'elemento con il focus (come in un gestore di file):
   * la raccolta lo usa come ancora se non ne ha già una.
   */
  ensureAnchor?: (id: Id) => void;
  selectAll?: () => void;
  invert?: () => void;
  clear?: () => void;
  /**
   * Destinazione di un tasto di spostamento per griglie irregolari (es. righe che ripartono a
   * ogni sessione): `undefined` usa il calcolo standard, `null` ignora il tasto.
   */
  targetOf?: (key: string, cursor: number) => number | null | undefined;
}

const isMod = (e: { ctrlKey: boolean; metaKey: boolean }) => e.ctrlKey || e.metaKey;

/** Nuova posizione per un tasto di spostamento, `null` se il tasto non sposta. */
export function navigationTarget(
  key: string,
  o: Pick<SelectionKeysOptions<Key>, "layout" | "columns" | "pageSize" | "count" | "cursor">,
): number | null {
  if (o.count <= 0) return null;
  const cols = o.layout === "grid" ? Math.max(1, o.columns ?? 1) : 1;
  const page = Math.max(1, o.pageSize);
  const cur = o.cursor;
  const none = cur < 0;
  let to: number;
  switch (key) {
    case "ArrowLeft":
      if (o.layout !== "grid") return null;
      to = none ? 0 : cur - 1;
      break;
    case "ArrowRight":
      if (o.layout !== "grid") return null;
      to = none ? 0 : cur + 1;
      break;
    case "ArrowUp":
      to = none ? o.count - 1 : cur - cols;
      break;
    case "ArrowDown":
      to = none ? 0 : cur + cols;
      break;
    case "PageUp":
      to = none ? 0 : cur - page;
      break;
    case "PageDown":
      to = none ? 0 : cur + page;
      break;
    case "Home":
      to = 0;
      break;
    case "End":
      to = o.count - 1;
      break;
    default:
      return null;
  }
  // In griglia le frecce su e giù non escono dalla colonna quando sono già sul bordo.
  if (!none && o.layout === "grid" && (key === "ArrowUp" || key === "ArrowDown")) {
    if (to < 0 || to > o.count - 1) return cur;
  }
  return Math.max(0, Math.min(o.count - 1, to));
}

/**
 * Gestisce un tasto della selezione; restituisce `true` (e chiama `preventDefault`) se lo ha
 * usato. Non fa nulla quando il focus è in un campo di testo o con Alt.
 */
export function handleSelectionKey<Id extends Key>(
  e: Pick<
    KeyboardEvent,
    "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey" | "target" | "preventDefault"
  > & { defaultPrevented?: boolean },
  o: SelectionKeysOptions<Id>,
): boolean {
  if (e.defaultPrevented || e.altKey || isTyping(e.target)) return false;
  const mod = isMod(e);
  const standard = navigationTarget(e.key, o);
  const custom = standard !== null && o.cursor >= 0 ? o.targetOf?.(e.key, o.cursor) : undefined;
  const to = custom === undefined ? standard : custom;
  if (to !== null) {
    e.preventDefault();
    if (e.shiftKey && o.cursor >= 0) {
      const from = o.idAt(o.cursor);
      if (from !== undefined) o.ensureAnchor?.(from);
    }
    o.moveTo(to);
    const id = o.idAt(to);
    if (id === undefined) return true;
    if (e.shiftKey) o.select(id, { ctrl: mod, shift: true });
    else if (!mod) o.select(id, { ctrl: false, shift: false });
    // Ctrl senza Maiusc: solo il focus.
    return true;
  }
  if (mod && (e.key === " " || e.key === "Spacebar")) {
    const id = o.cursor >= 0 ? o.idAt(o.cursor) : undefined;
    if (id === undefined) return false;
    e.preventDefault();
    o.select(id, { ctrl: true, shift: false });
    return true;
  }
  if (mod && !e.shiftKey && (e.key === "a" || e.key === "A") && o.selectAll) {
    e.preventDefault();
    o.selectAll();
    return true;
  }
  if (mod && e.shiftKey && (e.key === "i" || e.key === "I") && o.invert) {
    e.preventDefault();
    o.invert();
    return true;
  }
  if (e.key === "Escape" && !mod && !e.shiftKey && o.clear) {
    e.preventDefault();
    o.clear();
    return true;
  }
  return false;
}

/** `onKeyDown` per il contenitore della raccolta, con le opzioni sempre aggiornate. */
export function useSelectionKeys<Id extends Key>(options: SelectionKeysOptions<Id>) {
  const latest = useRef(options);
  latest.current = options;
  const ref = useRef((e: KeyboardEvent) => handleSelectionKey(e, latest.current));
  return ref.current;
}
