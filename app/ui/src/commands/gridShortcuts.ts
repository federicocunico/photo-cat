// Scorciatoie di selezione centralizzate (design D1 di ui-ux-overhaul, D4 di unified-selection):
// Ctrl/Cmd+A, Ctrl/Cmd+Maiusc+I ed Esc sono registrate una sola volta nella shell e inoltrate al
// **riquadro attivo**, cioè la raccolta in cui l'utente ha fatto l'ultimo clic o ha portato il
// focus. Il registro dei comandi le ignora nei campi di testo e con un dialogo aperto.
import { useEffect, useRef, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { useCommands } from "./registry";

/** Cosa una raccolta espone alle scorciatoie e ai menu. */
export interface ActivePane {
  /** Nome della raccolta per i menu e i lettori di schermo, es. "Volti". */
  label?: string;
  selectAll: () => unknown;
  invert: () => unknown;
  clear: () => unknown;
  /** Elementi selezionati ora: Esc passa alla raccolta solo se c'è qualcosa da togliere. */
  count: () => number;
}

type Entry = {
  pane: () => ActivePane;
  element: () => HTMLElement | null;
  /** Ultima interazione (clic o focus) dentro il riquadro; 0 se mai. */
  touched: number;
  /** Ordine di registrazione, per scegliere quando nessuno è stato toccato. */
  mounted: number;
};

const entries: Entry[] = [];
let tick = 0;

function pick(): Entry | undefined {
  let best: Entry | undefined;
  for (const e of entries) {
    if (!best) best = e;
    else if (e.touched !== best.touched ? e.touched > best.touched : e.mounted > best.mounted)
      best = e;
  }
  return best;
}

/** Aggiorna `data-active-pane`: serve solo quando i riquadri visibili sono più di uno. */
function mark() {
  const active = pick();
  const many = entries.filter((e) => e.element()).length > 1;
  for (const e of entries) {
    const el = e.element();
    if (!el) continue;
    if (many && e === active) el.setAttribute("data-active-pane", "");
    else el.removeAttribute("data-active-pane");
  }
}

/** Il riquadro attivo: l'ultimo toccato, altrimenti l'ultimo montato. */
export function activePane(): ActivePane | undefined {
  return pick()?.pane();
}

/**
 * Porta il focus della tastiera sulla raccolta del riquadro attivo (l'elemento con
 * `role="listbox"` o `role="grid"` focalizzabile, o il riquadro stesso) e la restituisce.
 */
export function focusActivePane(): HTMLElement | null {
  const el = pick()?.element();
  if (!el) return null;
  const sel = '[role="listbox"][tabindex], [role="grid"][tabindex]';
  const target = el.matches(sel) ? el : el.querySelector<HTMLElement>(sel);
  target?.focus();
  return target ?? null;
}

/**
 * Un solo ascoltatore globale, in fase di cattura: il riquadro diventa attivo prima che il clic
 * arrivi agli elementi, e vale anche per elementi montati dopo la registrazione (es. dopo un
 * caricamento).
 */
let listening = false;
function onInteract(e: Event) {
  const target = e.target;
  if (!(target instanceof Node)) return;
  // Il riquadro più interno che contiene il bersaglio (riquadri annidati: vince il più vicino).
  let hit: Entry | undefined;
  for (const entry of entries) {
    const el = entry.element();
    if (el?.contains(target) && (!hit || hit.element()?.contains(el))) hit = entry;
  }
  if (!hit || hit.touched === tick) return;
  hit.touched = ++tick;
  mark();
}
function listen() {
  if (listening || typeof document === "undefined") return;
  listening = true;
  for (const type of ["pointerdown", "mousedown", "click", "focusin"])
    document.addEventListener(type, onInteract, true);
}

/**
 * Registra una raccolta come riquadro finché il componente è montato. Con `ref`, un clic o il
 * focus dentro l'elemento la rendono il riquadro attivo; senza, la raccolta è attiva solo se è
 * l'ultima montata e nessun altro riquadro è stato toccato.
 */
export function useActivePane(
  pane: ActivePane,
  ref?: RefObject<HTMLElement | null>,
  enabled = true,
) {
  const latest = useRef(pane);
  latest.current = pane;
  const refRef = useRef(ref);
  refRef.current = ref;
  useEffect(() => {
    if (!enabled) return;
    const entry: Entry = {
      pane: () => latest.current,
      element: () => refRef.current?.current ?? null,
      touched: 0,
      mounted: ++tick,
    };
    entries.push(entry);
    listen();
    mark();
    return () => {
      entry.element()?.removeAttribute("data-active-pane");
      const i = entries.indexOf(entry);
      if (i >= 0) entries.splice(i, 1);
      mark();
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
      label: t("menu.selectAll"),
      group: "selection",
      shortcut: { key: "a", mod: true },
      menu: { menu: "edit", section: 1, order: 0 },
      disabledReason: () => (activePane() == null ? t("menu.why.noPane") : undefined),
      when: () => activePane() != null && !overlayOpen(),
      run: () => activePane()?.selectAll(),
    },
    {
      id: "selection.invert",
      label: t("menu.invertSelection"),
      group: "selection",
      shortcut: { key: "i", mod: true, shift: true },
      menu: { menu: "edit", section: 1, order: 1 },
      disabledReason: () => (activePane() == null ? t("menu.why.noPane") : undefined),
      when: () => activePane() != null && !overlayOpen(),
      run: () => activePane()?.invert(),
    },
    {
      id: "selection.clearActive",
      label: t("commands.clearSelection"),
      group: "selection",
      shortcut: { key: "Escape" },
      menu: { menu: "edit", section: 1, order: 2 },
      disabledReason: () =>
        (activePane()?.count() ?? 0) > 0 ? undefined : t("menu.why.noSelection"),
      // Un menu, un selettore o il visore aperto si chiude prima con il suo Esc.
      when: () => (activePane()?.count() ?? 0) > 0 && !overlayOpen(),
      run: () => activePane()?.clear(),
    },
  ]);
}
