// Accesso al backend: comandi generati (bindings.ts) ed eventi.
// I test sostituiscono questo modulo con vi.mock.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BootStatus,
  Config,
  ErrorBatch,
  JobSnapshot,
  LogLine,
  ModelStatus,
  ResourceSnapshot,
} from "../bindings";

import { commands as generated } from "../bindings";
import { trackCommand } from "../store/pending";
export const commands: typeof generated = new Proxy(generated, {
  get(target, key: keyof typeof generated) {
    const command = target[key];
    return (...args: unknown[]) =>
      trackCommand(key, () => (command as (...args: unknown[]) => Promise<unknown>)(...args));
  },
});
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
  errorBatch: "errors://batch",
  models: "models://status",
  uiRecovered: "app://ui-recovered",
  trimCaches: "app://trim-caches",
  facesChanged: "faces://changed",
} as const;

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function on<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => {});
  return listen<T>(name, (e) => cb(e.payload));
}

export const events = {
  onBootPhase: (cb: (s: BootStatus) => void) => on("boot://phase", cb),
  onBootReady: (cb: (s: BootStatus) => void) => on("boot://ready", cb),
  onJobProgress: (cb: (s: JobSnapshot) => void) => on(EVENTS.jobProgress, cb),
  onResources: (cb: (s: ResourceSnapshot) => void) => on(EVENTS.resources, cb),
  onLogLines: (cb: (l: LogLine[]) => void) => on(EVENTS.logLines, cb),
  onErrorBatch: (cb: (b: ErrorBatch) => void) => on(EVENTS.errorBatch, cb),
  onModels: (cb: (m: ModelStatus[]) => void) => on(EVENTS.models, cb),
  onUiRecovered: (cb: () => void) => on(EVENTS.uiRecovered, cb),
  onTrimCaches: (cb: () => void) => on(EVENTS.trimCaches, cb),
  onFacesChanged: (cb: () => void) => on(EVENTS.facesChanged, cb),
};
