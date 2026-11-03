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

/** Scorciatoie del mockup Persone; disattivate nei campi di testo. */
export function usePeopleHotkeys(actions: PeopleHotkeyActions, enabled = true) {
  const ref = useRef(actions);
  ref.current = actions;

  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || isTyping(e.target)) return;
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
          e.preventDefault();
          a.navigateGroup(-1);
          break;
        case "ArrowDown":
          e.preventDefault();
          a.navigateGroup(1);
          break;
        case " ":
        case "Spacebar":
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
