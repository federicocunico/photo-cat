// Tasti 1–9 e 0 del set attivo sulla selezione corrente (Libreria e Sequenza).
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { BulkAction, Target } from "../../lib/ipc";
import { activeSet, useTagOverview } from "./data";
import { isTyping } from "../../lib/keys";
import { useBulkJob } from "../library/bulkJob";

export { isTyping };

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
export function useTagHotkeys(selection: () => { target: Target; count: number }, enabled = true) {
  const { t } = useTranslation();
  const set = activeSet(useTagOverview().data);
  // Esito, "Annulla operazione" ed eventuali errori nell'unico ToastHost.
  const bulk = useBulkJob();
  const ref = useRef({ set, selection, bulk, t });
  ref.current = { set, selection, bulk, t };

  useEffect(() => {
    if (!enabled) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || isTyping(e.target)) return;
      const action = hotkeyAction(e.key, ref.current.set);
      if (!action) return;
      const { target, count } = ref.current.selection();
      if (!count) return;
      e.preventDefault();
      const { bulk, t } = ref.current;
      const message =
        action.action === "add_tag"
          ? t("selection.done.add_tag", { name: action.name, count })
          : t("selection.done.remove_tags", { count });
      void bulk.start(target, action, message);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled]);
}
