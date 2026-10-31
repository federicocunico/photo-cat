import { useTranslation } from "react-i18next";
import type { ErrorEntry, ErrorQuery } from "../../lib/ipc";
import type { DiagFilter } from "../../store/layout";

/** Messaggio leggibile: traduzione del codice se disponibile, altrimenti il testo registrato. */
export function useErrorText() {
  const { t, i18n } = useTranslation();
  return (e: Pick<ErrorEntry, "code" | "message">) => {
    const key = e.code ? `errors.${e.code.toLowerCase()}` : "";
    const fromCode = key && i18n.exists(key) ? t(key) : null;
    const fromMessage =
      e.message.startsWith("errors.") && i18n.exists(e.message) ? t(e.message) : null;
    return fromCode ?? fromMessage ?? e.message;
  };
}

export function toQuery(f: DiagFilter): ErrorQuery {
  return {
    level: f.level ?? null,
    phase: f.phase ?? null,
    jobId: f.jobId ?? null,
    code: null,
    text: f.text?.trim() ? f.text.trim() : null,
    limit: 500,
    offset: 0,
  };
}
