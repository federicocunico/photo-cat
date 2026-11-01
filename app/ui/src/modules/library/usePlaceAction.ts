// Azione sul luogo (job bulk "set_place") con aggiornamento di dettagli,
// griglia e facet al termine.
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { commands, type PlaceInput } from "../../lib/ipc";
import { useLive } from "../../store/live";

const FINAL = ["completed", "completed_with_errors", "cancelled", "failed"];

export function usePlaceAction(onDone?: () => void) {
  const qc = useQueryClient();
  const [job, setJob] = useState<number | null>(null);
  const [error, setError] = useState("");
  const result = useLive((s) => (job === null ? undefined : s.jobs[job]));
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    if (result && FINAL.includes(result.state)) {
      void qc.invalidateQueries({ queryKey: ["media-detail"] });
      void qc.invalidateQueries({ queryKey: ["library"] });
      void qc.invalidateQueries({ queryKey: ["facets"] });
      void qc.invalidateQueries({ queryKey: ["bulk-undo"] });
      done.current?.();
    }
  }, [result, qc]);
  const apply = async (ids: number[], place: PlaceInput | null) => {
    setError("");
    try {
      setJob(await commands.startBulk(ids, { action: "set_place", place }, false));
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  return { apply, error, result };
}
