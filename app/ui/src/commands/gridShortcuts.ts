// Scorciatoie di selezione centralizzate (design D1): Ctrl/Cmd+A, Ctrl/Cmd+Maiusc+I ed Esc
// vengono registrate una sola volta nella shell e inoltrate alla griglia attiva. Il registro
// dei comandi le ignora nei campi di testo e con un dialogo aperto (gli unici a ricevere
// anche lì i tasti sono i comandi con `inInputs`, qui nessuno).
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useCommands } from "./registry";

/** Cosa una griglia espone alle scorciatoie. */
export interface ActiveGrid {
  selectAll: () => unknown;
  invert: () => unknown;
  clear: () => unknown;
  /** Elementi selezionati ora: Esc passa alla griglia solo se c'è qualcosa da togliere. */
  count: () => number;
  /**
   * Per una vista secondaria montata accanto alla griglia principale (l'elenco delle persone
   * accanto ai volti): la vista è attiva **solo** mentre questa funzione è vera, cioè quando
   * l'utente ci sta lavorando. Le griglie senza `hasFocus` (Libreria, Sequenza, volti) restano
   * le uniche candidate quando nessuna vista secondaria ha il focus, nell'ordine di sempre:
   * così l'ordine in cui le viste si registrano non decide più chi riceve Ctrl+A.
   */
  hasFocus?: () => boolean;
}

type Entry = ActiveGrid & { focusOnly: boolean };

const active: Entry[] = [];

/**
 * La griglia attiva: una vista secondaria con il focus, altrimenti l'ultima griglia principale
 * registrata (montata più di recente).
 */
export function activeGrid(): ActiveGrid | undefined {
  for (let i = active.length - 1; i >= 0; i--) {
    const e = active[i]!;
    if (e.focusOnly && e.hasFocus?.()) return e;
  }
  for (let i = active.length - 1; i >= 0; i--) if (!active[i]!.focusOnly) return active[i];
  return undefined;
}

/** Rende `grid` la griglia attiva finché il componente è montato. */
export function useActiveGrid(grid: ActiveGrid, enabled = true) {
  const latest = useRef(grid);
  latest.current = grid;
  useEffect(() => {
    if (!enabled) return;
    const entry: Entry = {
      selectAll: () => latest.current.selectAll(),
      invert: () => latest.current.invert(),
      clear: () => latest.current.clear(),
      count: () => latest.current.count(),
      hasFocus: () => latest.current.hasFocus?.() ?? false,
      focusOnly: typeof latest.current.hasFocus === "function",
    };
    active.push(entry);
    return () => {
      const i = active.indexOf(entry);
      if (i >= 0) active.splice(i, 1);
    };
  }, [enabled]);
}

/**
 * Visore o menu aperto sopra la griglia: le scorciatoie di selezione non agiscono su una griglia
 * che l'utente non vede. Le finestre modali sono già escluse dal registro (`modalOpen`); la
 * palette dei comandi no, perché da lì le voci si eseguono anche con la palette aperta.
 */
const overlayOpen = () =>
  document.querySelector('.viewer[role="dialog"], [data-radix-popper-content-wrapper]') != null;

/** Da montare una volta nella shell. */
export function useGridShortcuts() {
  const { t } = useTranslation();
  useCommands([
    {
      id: "selection.all",
      label: t("selection.selectAll"),
      group: "selection",
      shortcut: { key: "a", mod: true },
      when: () => activeGrid() != null && !overlayOpen(),
      run: () => activeGrid()?.selectAll(),
    },
    {
      id: "selection.invert",
      label: t("selection.invert"),
      group: "selection",
      shortcut: { key: "i", mod: true, shift: true },
      when: () => activeGrid() != null && !overlayOpen(),
      run: () => activeGrid()?.invert(),
    },
    {
      id: "selection.clearActive",
      label: t("commands.clearSelection"),
      group: "selection",
      shortcut: { key: "Escape" },
      // Un menu, un selettore o il visore aperto si chiude prima con il suo Esc.
      when: () => (activeGrid()?.count() ?? 0) > 0 && !overlayOpen(),
      run: () => activeGrid()?.clear(),
    },
  ]);
}
