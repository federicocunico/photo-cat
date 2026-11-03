import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands } from "../../lib/ipc";
import { useLive } from "../../store/live";
import { useLibrary } from "../../store/library";
import { notify } from "../../store/toasts";
import { notifyUndoable } from "../../store/undo";
import { useCatalogOperation as useAction } from "./useCatalogAction";

/** Esiti di esclusioni e reinserimenti come avvisi nell'unico ToastHost. */
export function useCatalogFeedback() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const state = useAction();
  const result = useLive((s) => (state.job == null ? undefined : s.jobs[state.job]));
  const refresh = () => {
    for (const key of [
      "library",
      "library-count",
      "catalogs",
      "facets",
      "media-detail",
      "bulk-undo",
    ])
      void qc.invalidateQueries({ queryKey: [key] });
  };
  useEffect(() => {
    if (
      !result ||
      !["completed", "completed_with_errors", "failed", "cancelled"].includes(result.state)
    )
      return;
    refresh();
    useLibrary.getState().setSelection([], null);
    useAction.setState({
      busy: false,
      job: null,
      undoable: result.state === "completed",
      message:
        result.state === "completed"
          ? t(state.restore ? "exclusions.restored" : "exclusions.done", {
              count: state.count,
              name: state.catalog,
            })
          : t("exclusions.failed"),
    });
    // La query viene invalidata solo al termine del job seguito.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [result]);
  // L'esito va nell'unico ToastHost; un nuovo esito sostituisce il precedente. Le
  // operazioni annullabili passano dal registro globale: l'avviso e Ctrl+Z annullano lo stesso.
  useEffect(() => {
    if (!state.message) return;
    const message = state.message;
    if (state.undoable)
      notifyUndoable({
        key: "catalog-operation",
        message,
        run: async () => {
          await commands.undoBulk();
          refresh();
        },
      });
    else notify({ key: "catalog-operation", message, tone: "info" });
    // Consumato: lo stesso esito potrà ricomparire alla prossima operazione.
    useAction.setState({ message: "", undoable: false });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.message, state.undoable]);
}
