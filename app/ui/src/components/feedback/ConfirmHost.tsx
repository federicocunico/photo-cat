import { ConfirmDialog } from "../ui";
import { answerConfirm, useConfirm } from "../../store/confirm";

/** L'unico dialogo per le conferme chieste con `askConfirm`. */
export function ConfirmHost() {
  const request = useConfirm((s) => s.request);
  return (
    <ConfirmDialog
      open={request != null}
      onOpenChange={(open) => !open && answerConfirm(false)}
      title={request?.title ?? ""}
      message={request?.message}
      confirmLabel={request?.confirmLabel ?? ""}
      count={request?.count}
      files={request?.files}
      danger={request?.danger ?? true}
      onConfirm={() => answerConfirm(true)}
    />
  );
}
