// Fine di uno sviluppo dalla fotocamera: la sessione aperta si aggiorna da sola con i nuovi pannelli
// (stesso id, scelte intatte, nessuna riapertura della cartella) e un avviso dà il risultato. Sta
// a livello di app, non della pagina: lo sviluppo continua anche se si cambia modulo.
import { useEffect } from "react";
import i18n from "../../i18n";
import { events, type RawReviewDeveloped } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { useRawReview } from "../../store/rawReview";
import { notify } from "../../store/toasts";

/** Sostituisce la sessione aperta con quella riletta, restando sullo scatto che si stava guardando. */
export function refreshOpened(payload: RawReviewDeveloped): boolean {
  const { opened, shotIndex } = useRawReview.getState();
  if (!payload.opened || !opened || opened.sessionId !== payload.sessionId) return false;
  const currentKey = opened.shots[shotIndex]?.key;
  const index = payload.opened.shots.findIndex((s) => s.key === currentKey);
  // `setOpened` riparte dall'ultimo scatto salvato: qui si resta sullo scatto mostrato.
  useRawReview.setState({ opened: payload.opened, shotIndex: Math.max(0, index) });
  return true;
}

export function onDeveloped(payload: RawReviewDeveloped): void {
  const t = i18n.t.bind(i18n);
  const refreshed = refreshOpened(payload);
  const open = () => useLayout.getState().openModule({ module: "raw-review" });
  if (payload.refreshError) {
    notify({
      key: "raw-review-developed",
      tone: "error",
      message: t("rawDevelop.event.refreshError", { error: payload.refreshError }),
      action: { label: t("rawDevelop.event.open"), run: open },
    });
    return;
  }
  const result = t(
    payload.cancelled
      ? "rawDevelop.event.cancelled"
      : payload.failed > 0
        ? "rawDevelop.event.withErrors"
        : "rawDevelop.event.done",
    { converted: payload.converted, skipped: payload.skipped, failed: payload.failed },
  );
  notify({
    key: "raw-review-developed",
    tone: payload.failed > 0 ? "error" : "info",
    message: refreshed ? `${result} ${t("rawDevelop.event.refreshed")}` : result,
    action: refreshed ? undefined : { label: t("rawDevelop.event.open"), run: open },
  });
}

/** Ascolta `raw-review://developed` per tutta la vita dell'app. */
export function useDevelopedEvents(): void {
  useEffect(() => {
    const un = events.onRawReviewDeveloped(onDeveloped);
    return () => void un.then((f) => f()).catch(() => {});
  }, []);
}
