import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "../../../lib/ipc";
import type { JobSnapshot } from "../../../lib/ipc";
import { useLive } from "../../../store/live";

export const AI_OVERVIEW_KEY = ["ai", "overview"] as const;
const THROTTLE_MS = 2000;
const ACTIVE = ["queued", "running", "paused"];

/** Attività che cambiano la copertura delle analisi. */
const AI_KINDS = new Set(["analysis", "similar", "phash_backfill", "retune", "place_geo"]);

/** Un'importazione conta solo mentre ha una fase di analisi AI non conclusa. */
export function isAiJob(j: JobSnapshot): boolean {
  if (AI_KINDS.has(j.kind)) return true;
  return j.kind === "import" && j.phases.some((p) => p.name === "analisi_ai" && p.state !== "done");
}

export const isActiveJob = (j: JobSnapshot) => ACTIVE.includes(j.state);

/**
 * Copertura per fase, ricaricata al più ogni ~2 s mentre avanzano i job AI e una volta
 * subito al termine di un job (ai-workspace D7).
 */
export function useAiOverview() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: AI_OVERVIEW_KEY,
    queryFn: () => commands.aiOverview(),
  });
  useEffect(() => {
    let last = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const fire = () => {
      if (timer) clearTimeout(timer);
      timer = undefined;
      last = Date.now();
      void qc.invalidateQueries({ queryKey: AI_OVERVIEW_KEY });
    };
    const unsub = useLive.subscribe((st, prev) => {
      const changed = Object.values(st.jobs).filter(
        (j) => prev.jobs[j.id] !== j && (isAiJob(j) || (prev.jobs[j.id] && isAiJob(prev.jobs[j.id]))),
      );
      if (changed.length === 0) return;
      if (changed.some((j) => !isActiveJob(j))) return fire();
      if (timer) return;
      timer = setTimeout(fire, Math.max(0, THROTTLE_MS - (Date.now() - last)));
    });
    return () => {
      unsub();
      if (timer) clearTimeout(timer);
    };
  }, [qc]);
  return query;
}

/** Job AI attivi, il più recente per primo. */
export function useActiveAiJobs(): JobSnapshot[] {
  const jobs = useLive((s) => s.jobs);
  return Object.values(jobs)
    .filter((j) => isActiveJob(j) && isAiJob(j))
    .sort((a, b) => b.id - a.id);
}
