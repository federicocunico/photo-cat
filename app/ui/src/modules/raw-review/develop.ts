// Logica pura dello sviluppo dalla fotocamera (porting di `variants.ts` di FujiView): scelta dei
// preset, motivo per cui "Avvia sviluppo" non è ancora possibile e registro della fotocamera.
// Spazio e tempo si formattano con `fmtBytes` e `fmtDuration` di `lib/format`.
import type {
  CameraDiagnostics,
  CameraLogLine,
  CameraPreset,
  CameraState,
  CameraStatus,
  DiagnosticCheck,
  VariantPlan,
} from "../../lib/ipc";

/** Aggiunge o toglie un preset dalla scelta. */
export function togglePreset(selected: readonly string[], id: string): string[] {
  return selected.includes(id) ? selected.filter((s) => s !== id) : [...selected, id];
}

/**
 * Preset spuntati: la scelta fatta nel dialogo, altrimenti quella ricordata dal backend. Gli id
 * che la fotocamera non offre più si scartano; se la scelta ricordata non ne lascia nessuno, si
 * parte da tutti.
 */
export function effectiveSelection(
  presets: readonly CameraPreset[],
  remembered: readonly string[],
  touched: readonly string[] | null,
): string[] {
  const all = presets.map((p) => p.id);
  const wanted = touched ?? remembered;
  const kept = all.filter((id) => wanted.includes(id));
  if (kept.length > 0 || touched !== null) return kept;
  return all;
}

/** Perché lo sviluppo non può partire (chiave di `rawDevelop.blocks.<motivo>`). */
export type StartBlock =
  | "unavailable"
  | "camera_disconnected"
  | "camera_notReady"
  | "camera_busy"
  | "camera_error"
  | "camera_unresponsive"
  | "camera_unknown"
  | "checking"
  | "checks"
  | "noPresets"
  | "noShots"
  | "planning"
  | "planError"
  | "space"
  | "nothingToDo";

/** Il primo ostacolo all'avvio, o `null` se lo sviluppo può partire. */
export function startBlock(input: {
  status: CameraStatus | undefined;
  diagnostics: CameraDiagnostics | undefined;
  checking: boolean;
  presetCount: number;
  /** Scatti scelti; `null` = tutta la cartella. */
  shotCount: number | null;
  plan: VariantPlan | undefined;
  planning: boolean;
  planFailed: boolean;
}): StartBlock | null {
  const { status, diagnostics, plan } = input;
  if (!status) return "camera_unknown";
  if (!status.available) return "unavailable";
  if (status.state !== "ready") return `camera_${status.state}`;
  if (input.checking || !diagnostics || diagnostics.running) return "checking";
  if (!diagnostics.ready) return "checks";
  if (input.presetCount === 0) return "noPresets";
  if (input.shotCount === 0) return "noShots";
  if (input.planning) return "planning";
  if (input.planFailed || !plan) return "planError";
  if (!plan.enoughSpace) return "space";
  if (plan.pendingConversions <= 0) return "nothingToDo";
  return null;
}

/** Il controllo da sistemare per primo: gli errori prima degli avvisi. */
export function firstProblem(
  diagnostics: CameraDiagnostics | undefined,
): DiagnosticCheck | undefined {
  if (!diagnostics) return undefined;
  return (
    diagnostics.checks.find((c) => c.status === "error") ??
    diagnostics.checks.find((c) => c.status === "warning")
  );
}

/** Intestazione dell'elenco dei controlli (chiave di `rawCamera.headline.<tono>`). */
export type Headline = "checking" | "waiting" | "none" | "frozen" | "ready" | "problems";

export function headline(
  status: CameraStatus | undefined,
  diagnostics: CameraDiagnostics | undefined,
  checking: boolean,
): Headline {
  if (checking || diagnostics?.running) return "checking";
  if (status?.state === "unresponsive") return "waiting";
  if (diagnostics?.unresponsive) return "frozen";
  if (!diagnostics) return status?.state === "disconnected" ? "none" : "checking";
  if (diagnostics.ready) return "ready";
  return "problems";
}

/** Attesa prima del prossimo controllo leggero dello stato; `null` = nessun controllo. */
export function nextPollDelay(state: CameraState | undefined, hidden: boolean): number | null {
  if (hidden) return null;
  return state === "unresponsive" ? 15000 : 5000;
}

const two = (n: number) => String(n).padStart(2, "0");

export function formatClock(ms: number): string {
  const d = new Date(ms);
  return `${two(d.getHours())}:${two(d.getMinutes())}:${two(d.getSeconds())}`;
}

export function logToText(lines: readonly CameraLogLine[]): string {
  return lines.map((l) => `${formatClock(l.at)} [${l.level}] ${l.message}`).join("\n");
}

/** Aggiunge una riga al registro tenendo le ultime `limit` (come l'anello del backend). */
export function appendLog(
  lines: readonly CameraLogLine[],
  line: CameraLogLine,
  limit = 200,
): CameraLogLine[] {
  const next = [...lines, line];
  return next.length > limit ? next.slice(next.length - limit) : next;
}
