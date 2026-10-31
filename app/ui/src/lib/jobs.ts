import type { JobSnapshot } from "./ipc";

/** Fase da mostrare nella pill: la prima in corso, altrimenti la prima non completata. */
export function currentPhase(job: JobSnapshot) {
  return (
    job.phases.find((p) => p.state === "running" && p.percent != null && p.percent < 100) ??
    job.phases.find((p) => p.state !== "done") ??
    job.phases[job.phases.length - 1]
  );
}
