// Pulizia delle etichette: avvio del job e avviso "Annulla operazione" al termine.
// Il job viene seguito fuori dal componente, così l'avviso compare anche se l'utente
// lascia la sezione Pulizia prima che finisca.
import { useEffect, useState } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type JobSnapshot, type Target } from "../../../lib/ipc";
import { useLive } from "../../../store/live";
import { notify, notifyError } from "../../../store/toasts";
import { notifyUndoable } from "../../../store/undo";

const FINAL = ["completed", "completed_with_errors", "cancelled", "failed"];

/** Job di pulizia avviati e non ancora conclusi. */
const running = new Set<number>();
const listeners = new Set<() => void>();
const changed = () => listeners.forEach((l) => l());

function watch(
  job: number,
  message: string,
  failed: string,
  nothing: string,
  qc: QueryClient,
) {
  running.add(job);
  changed();
  const settle = (jobs: Record<number, JobSnapshot>) => {
    const j = jobs[job];
    if (!j || !FINAL.includes(j.state)) return false;
    running.delete(job);
    changed();
    void qc.invalidateQueries();
    if (j.state === "completed") {
      // Una pulizia senza modifiche non registra nulla: "Annulla" toccherebbe un'altra operazione.
      void commands
        .aiLastIsRejection()
        .catch(() => false)
        .then((rejected) => {
          if (!rejected) {
            notify({ key: "ai-cleanup", message: nothing });
            return;
          }
          notifyUndoable({
            key: "ai-cleanup",
            message,
            run: async () => {
              await commands.aiUndoRejection();
              await qc.invalidateQueries();
            },
          });
        });
    } else notify({ key: "ai-cleanup", tone: "error", message: failed });
    return true;
  };
  if (settle(useLive.getState().jobs)) return;
  const unsub = useLive.subscribe((st) => {
    if (settle(st.jobs)) unsub();
  });
}

export function useRejectLabels() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [busy, setBusy] = useState(running.size > 0);
  useEffect(() => {
    const l = () => setBusy(running.size > 0);
    listeners.add(l);
    l();
    return () => {
      listeners.delete(l);
    };
  }, []);

  const start = async (
    grp: string,
    value: string,
    maxScore: number | null,
    target: Target,
    message: string,
  ) => {
    try {
      const job = await commands.aiRejectLabels(grp, value, maxScore, target);
      watch(job, message, t("aiCleanup.failed"), t("aiCleanup.nothingToRemove"), qc);
      return job;
    } catch (error) {
      notifyError(error);
      return null;
    }
  };
  return { start, busy };
}
