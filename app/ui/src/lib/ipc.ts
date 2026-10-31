// Accesso al backend: comandi generati (bindings.ts) ed eventi.
// I test sostituiscono questo modulo con vi.mock.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Config,
  ErrorRecord,
  JobSnapshot,
  LogLine,
  ModelStatus,
  ResourceSnapshot,
} from "../bindings";

export { commands } from "../bindings";
export type * from "../bindings";

/**
 * Configurazione come la restituisce `get_config`: il backend serializza sempre ogni campo.
 * I binding la marcano opzionale solo per via di `#[serde(default)]` in lettura.
 */
export type Settings = {
  [K in keyof Config]-?: NonNullable<Config[K]> extends Record<string, unknown>
    ? Required<NonNullable<Config[K]>>
    : Config[K];
};

/** Nomi degli eventi: devono coincidere con app/src-tauri/src/events.rs. */
export const EVENTS = {
  jobProgress: "job://progress",
  resources: "resources://tick",
  logLines: "log://lines",
  error: "log://error",
  models: "models://status",
} as const;

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function on<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => {});
  return listen<T>(name, (e) => cb(e.payload));
}

export const events = {
  onJobProgress: (cb: (s: JobSnapshot) => void) => on(EVENTS.jobProgress, cb),
  onResources: (cb: (s: ResourceSnapshot) => void) => on(EVENTS.resources, cb),
  onLogLines: (cb: (l: LogLine[]) => void) => on(EVENTS.logLines, cb),
  onError: (cb: (e: ErrorRecord) => void) => on(EVENTS.error, cb),
  onModels: (cb: (m: ModelStatus[]) => void) => on(EVENTS.models, cb),
};
