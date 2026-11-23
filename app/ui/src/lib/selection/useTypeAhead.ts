// Ricerca per iniziale (unified-selection, design D7): digitando dei caratteri entro un secondo
// l'uno dall'altro, focus e selezione passano al primo elemento il cui nome inizia così, senza
// distinguere maiuscole e accenti. Solo nelle raccolte senza scorciatoie a lettera singola
// (Cataloghi, elenco dei tag): non in Persone (N, M, I…) né nelle griglie di foto (1–9).
import { useRef, type KeyboardEvent } from "react";
import { isTyping } from "../keys";

/** Testo confrontabile: minuscolo e senza accenti. */
export const foldName = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

/** Pausa oltre la quale i caratteri digitati iniziano una nuova ricerca. */
export const TYPE_AHEAD_MS = 1000;

/**
 * Restituisce un gestore di `keydown`: con un carattere stampabile (senza Ctrl, Cmd o Alt) cerca
 * tra i nomi e chiama `onFind` con la posizione trovata; restituisce `true` se ha usato il tasto.
 */
export function useTypeAhead(names: readonly string[], onFind: (index: number) => void) {
  const buffer = useRef({ text: "", at: 0 });
  const latest = useRef({ names, onFind });
  latest.current = { names, onFind };
  return (
    e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "target" | "preventDefault">,
  ) => {
    if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return false;
    if (e.key.length !== 1 || e.key === " ") return false;
    const now = Date.now();
    const b = buffer.current;
    b.text = now - b.at > TYPE_AHEAD_MS ? e.key : b.text + e.key;
    b.at = now;
    const wanted = foldName(b.text);
    const index = latest.current.names.findIndex((n) => foldName(n).startsWith(wanted));
    if (index < 0) return false;
    e.preventDefault();
    latest.current.onFind(index);
    return true;
  };
}
