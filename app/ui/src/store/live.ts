// Stato "vivo" alimentato dagli eventi del backend: job, risorse, errori recenti.

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import { notify } from "./toasts";
import i18n from "../i18n";
import { commands, events, isTauri } from "../lib/ipc";
import type { ErrorBatch, ErrorRecord, JobSnapshot, LogLine, ResourceSnapshot } from "../lib/ipc";

const TAIL_MAX = 500;

interface LiveState {
  jobs: Record<number, JobSnapshot>;
  resources: ResourceSnapshot | null;
  /** Errori/avvisi ricevuti in questa sessione (per il contatore della barra di stato). */
  errorCount: number;
  lastError: ErrorRecord | null;
  tail: LogLine[];
  upsertJob: (s: JobSnapshot) => void;
  setJobs: (list: JobSnapshot[]) => void;
  setResources: (r: ResourceSnapshot) => void;
  /** Lotto di errori raggruppati (`errors://batch`): un solo aggiornamento per lotto. */
  addErrors: (b: ErrorBatch) => void;
  addLines: (l: LogLine[]) => void;
  setTail: (lines: LogLine[]) => void;
  resetErrors: () => void;
}

const sameJob = (a: JobSnapshot | undefined, b: JobSnapshot) =>
  a !== undefined && JSON.stringify(a) === JSON.stringify(b);

export const useLive = create<LiveState>()((set) => ({
  jobs: {},
  resources: null,
  errorCount: 0,
  lastError: null,
  tail: [],
  // Solo l'elemento cambiato: uno snapshot identico non provoca alcun aggiornamento.
  upsertJob: (s) =>
    set((st) => (sameJob(st.jobs[s.id], s) ? st : { jobs: { ...st.jobs, [s.id]: s } })),
  setJobs: (list) => set({ jobs: Object.fromEntries(list.map((j) => [j.id, j])) }),
  setResources: (resources) => set({ resources }),
  addErrors: (b) =>
    set((st) => ({
      errorCount:
        st.errorCount +
        b.counts.filter((c) => c.level === "ERROR").reduce((n, c) => n + c.count, 0),
      lastError: b.recent[b.recent.length - 1] ?? st.lastError,
    })),
  addLines: (l) => set((st) => ({ tail: [...st.tail, ...l].slice(-TAIL_MAX) })),
  setTail: (tail) => set({ tail: tail.slice(-TAIL_MAX) }),
  resetErrors: () => set({ errorCount: 0 }),
}));

/** Job attivi, il più recente per primo. */
export const activeJobs = (jobs: Record<number, JobSnapshot>) =>
  Object.values(jobs)
    .filter((j) => j.state === "running" || j.state === "paused" || j.state === "queued")
    .sort((a, b) => b.id - a.id);

/** Sottoscrive gli eventi del backend e carica lo stato iniziale. */
export function useLiveEvents() {
  useEffect(() => {
    const s = useLive.getState();
    const started = new Map<number, number>();
    const progress = (job: JobSnapshot) => {
      if (["queued", "running", "paused"].includes(job.state)) {
        if (!started.has(job.id)) started.set(job.id, Date.now());
      } else {
        const since = started.get(job.id);
        if (since !== undefined && Date.now() - since >= 1000)
          notify({
            message:
              job.state === "completed"
                ? i18n.t("feedback.done")
                : job.state === "completed_with_errors" || job.state === "failed"
                  ? i18n.t("feedback.failed")
                  : i18n.t(`jobs.states.${job.state}`),
          });
        started.delete(job.id);
      }
      s.upsertJob(job);
    };
    const unlisten = [
      events.onJobProgress(progress),
      events.onResources(s.setResources),
      events.onErrorBatch(s.addErrors),
      events.onLogLines(s.addLines),
    ];
    if (isTauri()) {
      commands
        .listJobs()
        .then(s.setJobs)
        .catch(() => {});
      commands
        .getResources()
        .then(s.setResources)
        .catch(() => {});
      commands
        .logTail()
        .then(s.setTail)
        .catch(() => {});
    }
    return () => {
      unlisten.forEach((p) => p.then((f) => f()));
    };
  }, []);
}

/**
 * Memoria della WebView oltre la quota (`app://trim-caches`): si eliminano i dati delle
 * query non mostrate. Le query attive restano e si ricaricano solo se servono.
 */
export function useTrimCaches() {
  const qc = useQueryClient();
  useEffect(() => {
    const off = events.onTrimCaches(() => {
      qc.getQueryCache()
        .getAll()
        .filter((q) => q.getObserversCount() === 0)
        .forEach((q) => qc.removeQueries({ queryKey: q.queryKey, exact: true }));
    });
    return () => {
      void off.then((f) => f());
    };
  }, [qc]);
}
