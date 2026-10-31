// Stato "vivo" alimentato dagli eventi del backend: job, risorse, errori recenti.

import { useEffect } from "react";
import { create } from "zustand";
import { commands, events, isTauri } from "../lib/ipc";
import type { ErrorRecord, JobSnapshot, LogLine, ResourceSnapshot } from "../lib/ipc";

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
  addError: (e: ErrorRecord) => void;
  addLines: (l: LogLine[]) => void;
  setTail: (lines: LogLine[]) => void;
  resetErrors: () => void;
}

export const useLive = create<LiveState>()((set) => ({
  jobs: {},
  resources: null,
  errorCount: 0,
  lastError: null,
  tail: [],
  upsertJob: (s) => set((st) => ({ jobs: { ...st.jobs, [s.id]: s } })),
  setJobs: (list) => set({ jobs: Object.fromEntries(list.map((j) => [j.id, j])) }),
  setResources: (resources) => set({ resources }),
  addError: (e) =>
    set((st) => ({
      errorCount: e.level === "ERROR" ? st.errorCount + 1 : st.errorCount,
      lastError: e,
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
    const unlisten = [
      events.onJobProgress(s.upsertJob),
      events.onResources(s.setResources),
      events.onError(s.addError),
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
