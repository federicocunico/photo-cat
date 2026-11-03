// Operazioni bulk della Libreria (ui-ux-overhaul 3.3): avvio, aggiornamento delle viste
// al termine ed esito nell'unico ToastHost, con "Annulla operazione" se annullabile.
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type BulkAction, type Target } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
import { notify, notifyError } from "../../store/toasts";
import { notifyUndoable } from "../../store/undo";
import { refreshTags } from "../tags/data";

const FINAL = ["completed", "completed_with_errors", "cancelled", "failed"];

/**
 * Azioni con annullamento lato backend (`undo_bulk`). Restano fuori `assign_person` e
 * `remove_library`: inventario in tasks.md, 1.5.
 */
const UNDOABLE = new Set<BulkAction["action"]>([
  "add_tag",
  "remove_tag",
  "remove_tags",
  "add_catalog",
  "exclude_from_catalog",
  "restore_to_catalog",
  "set_label",
  "set_place",
]);

export const isUndoable = (op: BulkAction) => UNDOABLE.has(op.action);

type Pending = { job: number; message: string; undoable: boolean };

/**
 * Avvia operazioni bulk e ne segue la fine. `message` è l'esito in parole, mostrato
 * quando l'attività termina; più operazioni possono essere in corso insieme.
 */
export function useBulkJob(onFinished?: (state: string) => void) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [pending, setPending] = useState<Pending[]>([]);
  const jobs = useLive((s) => s.jobs);
  const done = useRef(onFinished);
  done.current = onFinished;

  useEffect(() => {
    const finished = pending.filter((p) => jobs[p.job] && FINAL.includes(jobs[p.job].state));
    if (!finished.length) return;
    for (const key of ["library", "library-count", "catalogs", "facets", "media-detail"])
      void qc.invalidateQueries({ queryKey: [key] });
    void qc.invalidateQueries({ queryKey: ["last-undo"] });
    void refreshTags(qc);
    for (const p of finished) {
      const result = jobs[p.job];
      if (result.state === "completed" && p.undoable)
        notifyUndoable({
          key: "library-result",
          message: p.message,
          run: async () => {
            await commands.undoBulk();
            await qc.invalidateQueries();
          },
        });
      else if (result.state === "completed")
        notify({ key: "library-result", message: p.message, tone: "ok" });
      else
        notify({
          key: "library-result",
          tone: "error",
          // Esito parziale: quante riuscite e quante no (bulk-operations, "Esito parziale").
          message:
            result.state === "completed_with_errors"
              ? t("selection.partial", {
                  done: result.phases.reduce((n, ph) => n + ph.done, 0),
                  failed: result.failed,
                })
              : t("selection.failed", { failed: result.failed }),
          action: {
            label: t("common.details"),
            run: () =>
              useLayout.getState().openModule({
                module: "settings",
                section: "diagnostics",
                target: { jobId: p.job },
              }),
          },
        });
      done.current?.(result.state);
    }
    setPending((list) => list.filter((p) => !finished.includes(p)));
  }, [pending, jobs, qc, t]);

  const start = async (target: Target, operation: BulkAction, message: string) => {
    try {
      const job = await commands.startBulk(
        target,
        operation,
        operation.action === "remove_library",
      );
      setPending((list) => [...list, { job, message, undoable: isUndoable(operation) }]);
      return job;
    } catch (error) {
      notifyError(error);
      return null;
    }
  };
  return { start, busy: pending.length > 0 };
}
