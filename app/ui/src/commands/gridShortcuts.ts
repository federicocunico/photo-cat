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
}

const active: ActiveGrid[] = [];

/** La griglia attiva è l'ultima registrata (la vista montata più di recente). */
export function activeGrid(): ActiveGrid | undefined {
  return active[active.length - 1];
}

/** Rende `grid` la griglia attiva finché il componente è montato. */
export function useActiveGrid(grid: ActiveGrid, enabled = true) {
  const latest = useRef(grid);
  latest.current = grid;
  useEffect(() => {
    if (!enabled) return;
    const entry: ActiveGrid = {
      selectAll: () => latest.current.selectAll(),
      invert: () => latest.current.invert(),
      clear: () => latest.current.clear(),
      count: () => latest.current.count(),
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
