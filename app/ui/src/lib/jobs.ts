import type { TFunction } from "i18next";
import type { JobRow, JobSnapshot } from "./ipc";

/** Fase da mostrare nella pill: la prima in corso, altrimenti la prima non completata. */
export function currentPhase(job: JobSnapshot) {
  return (
    job.phases.find((p) => p.state === "running" && p.percent != null && p.percent < 100) ??
    job.phases.find((p) => p.state !== "done") ??
    job.phases[job.phases.length - 1]
  );
}

/** Tipi di attività con un titolo proprio (`jobs.kinds.*`). */
export const JOB_KINDS = [
  "import",
  "relink",
  "export",
  "organize",
  "sequence_export",
  "bulk",
  "analysis",
  "models",
  "similar",
  "face_quality",
  "retune",
  "time_backfill",
  "phash_backfill",
  "photo_quality",
] as const;

/** Attività che scrivono file sul disco: interromperle chiede conferma (ui-foundations). */
export const FILE_WRITING_KINDS: ReadonlySet<string> = new Set([
  "export",
  "organize",
  "sequence_export",
]);

type Params = Record<string, unknown> | null | undefined;

/** L'oggetto dell'attività in parole: cartella, destinazione o numero di foto. */
export function jobObject(
  kind: string,
  params: Params,
  sourcePath: (id: number) => string | undefined,
): string | undefined {
  if (!params || typeof params !== "object") return undefined;
  const p = params as Record<string, unknown>;
  switch (kind) {
    case "import":
      return typeof p.sourceId === "number" ? sourcePath(p.sourceId) : undefined;
    case "relink":
      return typeof p.newFolder === "string" ? p.newFolder : undefined;
    case "export":
    case "organize":
      return typeof p.destination === "string" ? p.destination : undefined;
    case "sequence_export": {
      const options = p.options as Record<string, unknown> | undefined;
      return typeof options?.destination === "string" ? options.destination : undefined;
    }
    case "bulk":
    case "analysis":
      return Array.isArray(p.ids) ? String(p.ids.length) : undefined;
    default:
      return undefined;
  }
}

/** Titolo in parole (tipo + oggetto), mai un identificatore interno. */
export function jobTitle(
  t: TFunction,
  kind: string,
  params: Params,
  sourcePath: (id: number) => string | undefined = () => undefined,
): string {
  if (!(JOB_KINDS as readonly string[]).includes(kind)) return t("jobs.kinds.other");
  const object = jobObject(kind, params, sourcePath);
  if (object != null && t(`jobs.kinds.${kind}_of`, { defaultValue: "" }))
    return t(`jobs.kinds.${kind}_of`, { object });
  return t(`jobs.kinds.${kind}`);
}

/** Parametri di un'attività dallo storico (le attività in corso vi compaiono già). */
export const paramsOf = (rows: JobRow[] | undefined, id: number): Params =>
  rows?.find((r) => r.id === id)?.params as Params;
