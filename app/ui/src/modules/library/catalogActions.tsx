import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands } from "../../lib/ipc";
import { useLive } from "../../store/live";
import { useLibrary } from "../../store/library";
import { Toast } from "../../components/feedback/Toast";
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
  if (!state.message) return null;
  return (
    <Toast key={state.message} onClose={closeToast}>
      {state.message}{" "}
      {state.undoable && (
        <button
          className="btn"
          disabled={state.busy}
          onClick={() => {
            if (useAction.getState().busy) return;
            useAction.setState({ busy: true });
            void commands
              .undoBulk()
              .then(() => {
                refresh();
                useAction.setState({ busy: false, message: "", undoable: false });
              })
              .catch((error) =>
                useAction.setState({ busy: false, message: String(error), undoable: false }),
              );
          }}
        >
          {t("exclusions.undo")}
        </button>
      )}
    </Toast>
  );
}
