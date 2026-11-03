// Dialoghi modali (Radix): focus intrappolato, Esc chiude, il focus torna a chi li ha aperti.
import * as RAlert from "@radix-ui/react-alert-dialog";
import * as RDialog from "@radix-ui/react-dialog";
import { useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { Button } from "./Button";

/**
 * Radix riporta il focus solo al proprio `Trigger`; i dialoghi dell'app sono quasi sempre
 * controllati (aperti da menu, scorciatoie, altri dialoghi), quindi si ricorda chi aveva il
 * focus all'apertura e lo si restituisce alla chiusura.
 */
function useReturnFocus(open: boolean) {
  const returnTo = useRef<HTMLElement | null>(null);
  const wasOpen = useRef(false);
  if (open && !wasOpen.current && typeof document !== "undefined") {
    returnTo.current = document.activeElement as HTMLElement | null;
  }
  wasOpen.current = open;
  return (e: Event) => {
    const el = returnTo.current;
    if (el && el !== document.body && el.isConnected) {
      e.preventDefault();
      el.focus();
    }
  };
}

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  width = 480,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  /** Pulsanti in fondo: il comando principale per ultimo. */
  footer?: ReactNode;
  width?: number;
}) {
  const { t } = useTranslation();
  const onCloseAutoFocus = useReturnFocus(open);
  return (
    <RDialog.Root open={open} onOpenChange={onOpenChange}>
      <RDialog.Portal>
        <RDialog.Overlay className="ui-overlay" />
        <RDialog.Content
          className="ui-dialog"
          style={{ width: `min(${width}px, calc(100vw - 32px))` }}
          onCloseAutoFocus={onCloseAutoFocus}
          {...(description ? {} : { "aria-describedby": undefined })}
        >
          <div className="ui-dialog-head">
            <RDialog.Title>{title}</RDialog.Title>
            <RDialog.Close asChild>
              <button
                type="button"
                className="btn btn-ghost icon-btn"
                aria-label={t("common.close")}
              >
                <Icon.Close size={16} />
              </button>
            </RDialog.Close>
          </div>
          {description && (
            <RDialog.Description className="muted">{description}</RDialog.Description>
          )}
          {children && <div className="ui-dialog-body">{children}</div>}
          {footer && <div className="ui-dialog-footer">{footer}</div>}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}

/**
 * Conferma di un'azione che non si può annullare, o che tocca i file. Il pulsante dice il
 * verbo ("Elimina 3 tag"), il testo dice quanti elementi e cosa succede ai file.
 * Il focus iniziale va su "Annulla", così Invio per sbaglio non conferma.
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  message,
  confirmLabel,
  count,
  files,
  danger = true,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  message?: ReactNode;
  /** Verbo esplicito, es. "Elimina catalogo". Mai "OK" o "Sì". */
  confirmLabel: string;
  /** Quanti elementi riguarda l'azione. */
  count?: number;
  /** Effetto sui file: `untouched` (solo catalogo) o `affected` (sposta/elimina file). */
  files?: "untouched" | "affected";
  danger?: boolean;
  onConfirm: () => unknown;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const onCloseAutoFocus = useReturnFocus(open);
  const confirm = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await onConfirm();
      onOpenChange(false);
    } finally {
      setBusy(false);
    }
  };
  return (
    <RAlert.Root open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
      <RAlert.Portal>
        <RAlert.Overlay className="ui-overlay" />
        <RAlert.Content
          className="ui-dialog"
          onCloseAutoFocus={onCloseAutoFocus}
          style={{ width: "min(440px, calc(100vw - 32px))" }}
        >
          <RAlert.Title>{title}</RAlert.Title>
          <RAlert.Description asChild>
            <div className="ui-dialog-body">
              {message && <p>{message}</p>}
              {count != null && <p className="muted">{t("common.confirmCount", { count })}</p>}
              {files && (
                <p className={files === "affected" ? "ui-text-danger" : "muted"}>
                  {t(files === "affected" ? "common.filesAffected" : "common.filesUntouched")}
                </p>
              )}
            </div>
          </RAlert.Description>
          <div className="ui-dialog-footer">
            <RAlert.Cancel asChild>
              <Button disabled={busy}>{t("common.cancel")}</Button>
            </RAlert.Cancel>
            <Button
              variant={danger ? "danger" : "primary"}
              busy={busy}
              onClick={(e) => {
                // Si chiude solo a operazione riuscita (un errore resta visibile nel dialogo).
                e.preventDefault();
                return confirm();
              }}
            >
              {confirmLabel}
            </Button>
          </div>
        </RAlert.Content>
      </RAlert.Portal>
    </RAlert.Root>
  );
}
