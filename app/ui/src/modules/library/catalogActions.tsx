import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands } from "../../lib/ipc";
import { useLive } from "../../store/live";
import { useLibrary } from "../../store/library";
import { notify, notifyError } from "../../store/toasts";
import { useCatalogOperation as useAction } from "./useCatalogAction";

const closeToast = () => useAction.setState({ message: "" });

export function CatalogFeedback() {
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
  // L'esito va nell'unico ToastHost; un nuovo esito sostituisce il precedente.
  useEffect(() => {
    if (!state.message) return;
    notify({
      key: "catalog-operation",
      message: state.message,
      tone: state.undoable ? "ok" : "info",
      onDismiss: closeToast,
      action: state.undoable
        ? {
            label: t("exclusions.undo"),
            run: async () => {
              if (useAction.getState().busy) return;
              useAction.setState({ busy: true });
              try {
                await commands.undoBulk();
                refresh();
                useAction.setState({ busy: false, undoable: false });
              } catch (error) {
                useAction.setState({ busy: false, undoable: false });
                notifyError(error);
              }
            },
          }
        : undefined,
    });
    // Solo al cambio del messaggio: le funzioni usate leggono lo stato al momento del clic.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.message, state.undoable]);
  return null;
}
