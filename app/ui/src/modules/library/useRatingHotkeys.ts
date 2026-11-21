// Alt+1–5 assegnano la valutazione e Alt+0 la tolgono, sulla selezione corrente
// (Libreria e Sequenza). I tasti 1–9/0 senza Alt restano ai tag (useTagHotkeys).
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import type { Target } from "../../lib/ipc";
import { useCommands } from "../../commands/registry";
import { useBulkJob } from "./bulkJob";

export function useRatingHotkeys(
  selection: () => { target: Target; count: number },
  enabled = true,
) {
  const { t } = useTranslation();
  const bulk = useBulkJob();
  const ref = useRef({ selection, bulk, t });
  ref.current = { selection, bulk, t };

  useCommands(
    [0, 1, 2, 3, 4, 5].map((n) => ({
      id: `rating.key.${n}`,
      label: n === 0 ? t("selection.rateKeyNone") : t("selection.rateKey", { count: n }),
      group: "selection" as const,
      shortcut: { key: String(n), alt: true },
      palette: false,
      when: () => ref.current.selection().count > 0,
      run: () => {
        const { target, count } = ref.current.selection();
        return ref.current.bulk.start(
          target,
          { action: "set_rating", rating: n === 0 ? null : n },
          ref.current.t("selection.done.set_rating", { count }),
        );
      },
    })),
    enabled,
  );
}
