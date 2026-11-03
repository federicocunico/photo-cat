import { useEffect, useRef } from "react";
import { isTyping } from "../tags/useTagHotkeys";

export type PeopleHotkeyActions = {
  /** ↑/↓: gruppo precedente / successivo nell'elenco. */
  navigateGroup: (delta: -1 | 1) => void;
  /** Spazio: seleziona/deseleziona il volto con il focus. */
  toggleFocusedFace: () => void;
  /** N: focus su «Chi è?». */
  focusWhoIs: () => void;
  /** M: focus su «Sposta in…». */
  focusMove: () => void;
  /** X: non è questa persona (rifiuta i volti selezionati / not_same). */
  notSame: () => void;
  /** U: unisci gruppo o righe selezionate. */
  merge: () => void;
  /** I: ignora il gruppo corrente. */
  ignore: () => void;
  /** Ctrl+Z: annulla l'ultima correzione. */
  undo: () => void;
};

/**
 * Una finestra modale aperta (unione, conferme) sospende le scorciatoie della pagina.
 * `.ui-dialog` è il `Dialog` condiviso (Radix non mette `aria-modal`).
 */
function modalOpen(): boolean {
  return (
    document.querySelector(
      '[role="dialog"][aria-modal="true"], [role="dialog"].ui-dialog, [role="alertdialog"]',
    ) != null
  );
}

/**
 * Spazio e Invio su un controllo interattivo (pulsante, casella, link, menu)
 * appartengono al controllo stesso, salvo che sia una miniatura di volto.
 */
function ownsActivation(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest("[data-face-tile]")) return false;
  return (
    target.closest(
      'button, a[href], select, input, [role="button"], [role="checkbox"], [role="menuitem"], [role="option"], [role="switch"], [role="tab"]',
    ) != null
  );
}

/** Le frecce dentro un menu, un elenco o un selettore restano a quel controllo. */
function ownsArrows(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    target.closest(
      'select, [role="listbox"], [role="menu"], [role="radiogroup"], [role="slider"], [role="tablist"]',
    ) != null
  );
}

/** Scorciatoie del mockup Persone; disattivate nei campi di testo e con finestre modali aperte. */
export function usePeopleHotkeys(actions: PeopleHotkeyActions, enabled = true) {
  const ref = useRef(actions);
  ref.current = actions;

  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || isTyping(e.target) || modalOpen()) return;
      const a = ref.current;
      const mod = e.ctrlKey || e.metaKey;
      if (mod && (e.key === "z" || e.key === "Z") && !e.shiftKey) {
        e.preventDefault();
        a.undo();
        return;
      }
      if (mod || e.altKey) return;
      switch (e.key) {
        case "ArrowUp":
          if (ownsArrows(e.target)) return;
          e.preventDefault();
          a.navigateGroup(-1);
          break;
        case "ArrowDown":
          if (ownsArrows(e.target)) return;
          e.preventDefault();
          a.navigateGroup(1);
          break;
        case " ":
        case "Spacebar":
          if (ownsActivation(e.target)) return;
          e.preventDefault();
          a.toggleFocusedFace();
          break;
        case "n":
        case "N":
          e.preventDefault();
          a.focusWhoIs();
          break;
        case "m":
        case "M":
          e.preventDefault();
          a.focusMove();
          break;
        case "x":
        case "X":
          e.preventDefault();
          a.notSame();
          break;
        case "u":
        case "U":
          e.preventDefault();
          a.merge();
          break;
        case "i":
        case "I":
          e.preventDefault();
          a.ignore();
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}
