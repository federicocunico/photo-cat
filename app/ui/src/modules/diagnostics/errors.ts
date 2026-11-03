import { useTranslation } from "react-i18next";
import type { ErrorEntry, ErrorQuery } from "../../lib/ipc";
import type { DiagFilter } from "../../store/layout";

/** File per pagina nell'elenco di un gruppo di errori. */
export const GROUP_PAGE = 50;

/** Fasi delle attività in cui il motore registra errori (filtro "Fase"). */
export const PHASES = [
  "scansione",
  "metadati",
  "miniature",
  "analisi_ai",
  "volti",
  "scrittura",
  "modelli",
];

/** Livelli registrati negli errori (dal motore, in maiuscolo). */
export const ENTRY_LEVELS = ["ERROR", "WARN"] as const;

/** Livelli del log dell'app, dal meno al più dettagliato. */
export const LOG_LEVELS = ["error", "warn", "info", "debug", "trace"] as const;

const CAUSES = ["truncated", "corrupt", "unsupported_variant", "no_raw_preview", "too_large", "io"];

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

/** Nomi tradotti di fase e livello di un errore (mai il valore grezzo del motore). */
export function useEntryLabels() {
  const { t, i18n } = useTranslation();
  return {
    phase: (p: string | null | undefined) => {
      if (!p) return "—";
      return i18n.exists(`jobs.phases.${p}`) ? t(`jobs.phases.${p}`) : t("diag.otherPhase");
    },
    level: (l: string) => {
      const key = `diag.levels.${l.toLowerCase()}`;
      return i18n.exists(key) ? t(key) : l;
    },
  };
}

/** Motivo leggibile di un errore di decodifica (chiave all'inizio del dettaglio), se presente. */
export function useCauseText() {
  const { t } = useTranslation();
  return (detail: string | null) => {
    const key = detail?.split(":")[0] ?? "";
    return CAUSES.includes(key) ? t(`diag.causes.${key}`) : null;
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
