// Tasti 1–9 e 0 del set attivo sulla selezione corrente (Libreria e Sequenza).
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { commands, type BulkAction } from "../../lib/ipc";
import { useLive } from "../../store/live";
import { activeSet, refreshTags, useTagOverview } from "./data";

const FINAL = ["completed", "completed_with_errors", "cancelled", "failed"];
const NON_TEXT_INPUTS = [
  "checkbox",
  "radio",
  "range",
  "button",
  "submit",
  "reset",
  "color",
  "file",
];

/** Vero se il fuoco è in un campo dove i tasti scrivono testo. */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target.closest("[contenteditable='true']")) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") return !NON_TEXT_INPUTS.includes((target as HTMLInputElement).type);
  return false;
}

/** Azione bulk per un tasto del set attivo, o `null` se il tasto non ha effetto. */
export function hotkeyAction(
  key: string,
  set: { tags: Array<{ key: number; name: string }> } | undefined,
): BulkAction | null {
  if (!set || !/^[0-9]$/.test(key)) return null;
  if (key === "0") {
    if (!set.tags.length) return null;
    return { action: "remove_tags", names: set.tags.map((t) => t.name) };
  }
  const tag = set.tags.find((t) => t.key === Number(key));
  return tag ? { action: "add_tag", name: tag.name } : null;
}

/**
 * Collega i tasti alla selezione restituita da `selection` (letta al momento
 * della pressione, così vale anche "tutti i risultati" non visibili).
 * Al termine del job aggiorna le viste senza ricaricare.
 */
export function useTagHotkeys(selection: () => number[], enabled = true) {
  const qc = useQueryClient();
  const set = activeSet(useTagOverview().data);
  const [job, setJob] = useState<number | null>(null);
  const result = useLive((s) => (job === null ? undefined : s.jobs[job]));
  const ref = useRef({ set, selection });
  ref.current = { set, selection };

  useEffect(() => {
    if (result && FINAL.includes(result.state)) {
      setJob(null);
      void refreshTags(qc);
    }
  }, [result, qc]);

  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || isTyping(e.target)) return;
      const action = hotkeyAction(e.key, ref.current.set);
      if (!action) return;
      const ids = ref.current.selection();
      if (!ids.length) return;
      e.preventDefault();
      void commands
        .startBulk(ids, action, false)
        .then(setJob)
        .catch(() => {});
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}
