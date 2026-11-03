// "Sposta nel cestino" condiviso da barra delle azioni, tasto Canc e trascinamento: se le
// foto stanno in cataloghi manuali chiede conferma (con l'elenco), altrimenti parte subito.
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "../../components/ui";
import { commands, type CatalogLoss, type Target } from "../../lib/ipc";
import { notifyError } from "../../store/toasts";
import { useBulkJob } from "./bulkJob";

type Asking = { target: Target; count: number; losses: CatalogLoss[] };

export function useMoveToTrash(): {
  moveToTrash: (target: Target, count: number) => Promise<void>;
  /** Da montare una volta nel componente che usa l'helper. */
  dialog: ReactNode;
} {
  const { t } = useTranslation();
  const bulk = useBulkJob();
  const [asking, setAsking] = useState<Asking | null>(null);

  const run = (target: Target, count: number) =>
    bulk.start(target, { action: "move_to_trash" }, t("selection.done.trash", { count }));

  const moveToTrash = async (target: Target, count: number) => {
    let losses: CatalogLoss[];
    try {
      losses = await commands.trashImpact(target);
    } catch (error) {
      notifyError(error);
      return;
    }
    if (losses.length === 0) {
      await run(target, count);
      return;
    }
    setAsking({ target, count, losses });
  };

  const dialog = (
    <ConfirmDialog
      open={asking != null}
      onOpenChange={(o) => !o && setAsking(null)}
      title={t("selection.trashConfirmTitle", { count: asking?.count ?? 0 })}
      message={
        asking && (
          <>
            {t("selection.trashConfirmIntro")}
            {asking.losses.map((l) => (
              <span key={l.name} style={{ display: "block" }}>
                {t("selection.trashConfirmLine", { name: l.name, count: l.items })}
              </span>
            ))}
          </>
        )
      }
      confirmLabel={t("selection.trashConfirm")}
      danger={false}
      focusConfirm
      onConfirm={async () => {
        if (asking) await run(asking.target, asking.count);
      }}
    />
  );
  return { moveToTrash, dialog };
}
