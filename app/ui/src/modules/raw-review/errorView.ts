// Un errore del backend (`AppError`) come lo mostra lo sviluppo dalla fotocamera: titolo tradotto
// dal codice (`errors.<codice>`), azione suggerita (`hints.<codice>`) e dettaglio tecnico del
// backend (in italiano), che resta per l'assistenza.
import type { TFunction } from "i18next";

export type ErrorView = {
  /** Messaggio tradotto, se il codice è noto. */
  title?: string;
  /** Cosa fare, se il codice ne ha una. */
  hint?: string;
  detail: string;
};

type Raw = { code?: unknown; detail?: unknown; message?: unknown } | null | undefined;

export function errorView(
  error: unknown,
  t: TFunction,
  exists: (key: string) => boolean,
): ErrorView {
  const raw = (typeof error === "object" ? error : undefined) as Raw;
  const code = typeof raw?.code === "string" ? raw.code.toLowerCase() : "";
  const detail =
    typeof raw?.detail === "string"
      ? raw.detail
      : typeof raw?.message === "string"
        ? raw.message
        : String(error);
  const titleKey = `errors.${code}`;
  const hintKey = `hints.${code}`;
  return {
    title: code && exists(titleKey) ? t(titleKey) : undefined,
    hint: code && exists(hintKey) ? t(hintKey) : undefined,
    detail,
  };
}

/** Come [`errorView`] per un codice e un motivo già separati (riepilogo dello sviluppo). */
export function codeView(
  code: string,
  detail: string,
  t: TFunction,
  exists: (key: string) => boolean,
): ErrorView {
  return errorView({ code, detail }, t, exists);
}

/** Il suggerimento per riprendere un'attività in pausa (`jobs.pauseHints.<codice>`). */
export function pauseHint(
  code: string | undefined,
  t: TFunction,
  exists: (key: string) => boolean,
): string | undefined {
  const key = `jobs.pauseHints.${code ?? ""}`;
  return code && exists(key) ? t(key) : undefined;
}
