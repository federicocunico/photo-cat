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

/** Da montare una volta nella shell. */
export function useGridShortcuts() {
  const { t } = useTranslation();
  useCommands([
    {
      id: "selection.all",
      label: t("selection.selectAll"),
      group: "selection",
      shortcut: { key: "a", mod: true },
      when: () => activeGrid() != null,
      run: () => activeGrid()?.selectAll(),
    },
    {
      id: "selection.invert",
      label: t("selection.invert"),
      group: "selection",
      shortcut: { key: "i", mod: true, shift: true },
      when: () => activeGrid() != null,
      run: () => activeGrid()?.invert(),
    },
    {
      id: "selection.clearActive",
      label: t("commands.clearSelection"),
      group: "selection",
      shortcut: { key: "Escape" },
      // Un menu o un selettore aperto si chiude prima con il suo Esc.
      when: () =>
        (activeGrid()?.count() ?? 0) > 0 &&
        !document.querySelector("[data-radix-popper-content-wrapper]"),
      run: () => activeGrid()?.clear(),
    },
  ]);
}
