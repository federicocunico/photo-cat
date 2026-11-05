import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "../../../lib/ipc";
import { useLive } from "../../../store/live";

export const SIMILAR_STATS_KEY = ["ai", "similar-stats"] as const;
const KINDS = new Set(["similar", "analysis", "phash_backfill"]);
const THROTTLE_MS = 2000;

/** Job di analisi dei gruppi in corso (il più recente). */
export function useActiveSimilarJob() {
  return useLive((s) =>
    Object.values(s.jobs)
      .filter((j) => j.kind === "similar" && ["queued", "running", "paused"].includes(j.state))
      .sort((a, b) => b.id - a.id)[0],
  );
}

/** Statistiche dei gruppi, ricaricate (al più ogni ~2 s) quando avanzano i job collegati. */
export function useSimilarStats() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: SIMILAR_STATS_KEY,
    queryFn: () => commands.aiSimilarStats(null),
  });
  useEffect(() => {
    let last = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const fire = () => {
      timer = undefined;
      last = Date.now();
      void qc.invalidateQueries({ queryKey: SIMILAR_STATS_KEY });
    };
    const unsub = useLive.subscribe((st, prev) => {
      const changed = Object.values(st.jobs).some((j) => KINDS.has(j.kind) && prev.jobs[j.id] !== j);
      if (!changed || timer) return;
      timer = setTimeout(fire, Math.max(0, THROTTLE_MS - (Date.now() - last)));
    });
    return () => {
      unsub();
      if (timer) clearTimeout(timer);
    };
  }, [qc]);
  return query;
}
