// Azioni e dati condivisi della gestione delle importazioni (senza componenti).
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { commands, isTauri, type Source } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { useLive } from "../../store/live";

export async function pickFolder(defaultPath?: string): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({ directory: true, multiple: false, defaultPath });
  return typeof res === "string" ? res : null;
}

export const TERMINAL = ["completed", "completed_with_errors", "cancelled", "failed"];

/** Job che cambiano sorgenti e libreria: al termine si ricaricano i dati collegati. */
const REFRESH_KINDS = ["import", "relink", "bulk"];
const REFRESH_KEYS = [
  "sources",
  "import-updates",
  "relink",
  "folders",
  "library",
  "library-count",
  "facets",
];

export function useImportRefresh() {
  const qc = useQueryClient();
  const jobs = useLive((s) => s.jobs);
  const seen = useRef(new Set<number>());
  useEffect(() => {
    let changed = false;
    for (const j of Object.values(jobs)) {
      if (!REFRESH_KINDS.includes(j.kind) || !TERMINAL.includes(j.state)) continue;
      if (seen.current.has(j.id)) continue;
      seen.current.add(j.id);
      changed = true;
    }
    if (!changed) return;
    for (const key of REFRESH_KEYS) void qc.invalidateQueries({ queryKey: [key] });
  }, [jobs, qc]);
}

export function useImportUpdates() {
  return useQuery({
    queryKey: ["import-updates"],
    queryFn: commands.importUpdates,
    enabled: isTauri(),
  });
}

/** Apre la Libreria sui soli elementi indicati. */
export function useShowInLibrary() {
  const setModule = useLayout((s) => s.setModule);
  return (ids: number[]) => {
    useLibrary.getState().setFilter({ ...emptyFilter(), ids });
    setModule("library");
  };
}

/** "Aggiorna cartella…": sceglie una sottocartella della sorgente e la aggiorna. */
export async function updateFolder(source: Source, folder?: string) {
  const picked = folder ?? (await pickFolder(source.path));
  if (!picked) return null;
  return commands.startImport(source.id, picked);
}
