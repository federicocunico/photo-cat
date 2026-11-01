// Pulizia della configurazione prima del salvataggio: righe vuote di
// argomenti e cartelle da ignorare, spazi superflui nei nomi.
import type { Settings } from "../../lib/ipc";

export function cleanSettings(cfg: Settings): Settings {
  return {
    ...cfg,
    external_apps: (cfg.external_apps ?? []).map((a) => ({
      ...a,
      name: (a.name ?? "").trim(),
      path: (a.path ?? "").trim(),
      args: (a.args ?? []).filter((x) => x.trim() !== ""),
    })),
    ai: {
      ...cfg.ai,
      ignored_folder_names: (cfg.ai.ignored_folder_names ?? [])
        .map((x) => x.trim())
        .filter(Boolean),
    },
  };
}
