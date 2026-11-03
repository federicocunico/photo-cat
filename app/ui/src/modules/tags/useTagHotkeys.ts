// Tasti 1–9 e 0 del set attivo sulla selezione corrente (Libreria e Sequenza).
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import type { BulkAction, Target } from "../../lib/ipc";
import { activeSet, useTagOverview } from "./data";
import { isTyping } from "../../lib/keys";
import { useCommands } from "../../commands/registry";
import { useBulkJob } from "../library/bulkJob";
import { useGuide } from "../../store/guide";

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

  useCommands(
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"].map((key) => {
      const action = hotkeyAction(key, set);
      return {
        id: `tags.key.${key}`,
        label:
          action?.action === "add_tag"
            ? t("commands.tagKey", { name: action.name })
            : t("commands.removeSetTags"),
        group: "selection" as const,
        shortcut: { key },
        // Il comando c'è solo se il tasto ha un tag e c'è una selezione.
        palette: false,
        when: () => action != null && ref.current.selection().count > 0,
        run: () => {
          if (!action) return;
          const { target, count } = ref.current.selection();
          if (action.action === "add_tag") useGuide.getState().complete("tags");
          const message =
            action.action === "add_tag"
              ? t("selection.done.add_tag", { name: action.name, count })
              : t("selection.done.remove_tags", { count });
          return ref.current.bulk.start(target, action, message);
        },
      };
    }),
    enabled,
  );
}
