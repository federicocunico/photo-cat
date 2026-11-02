// Long task dell'interfaccia (≥ 50 ms): contati dal browser e inviati al log ogni 5 s.
// Il backend li registra a livello debug ("modalità diagnostica"): il test di carico li legge dal log.

import { commands, isTauri } from "./ipc";

export const LONG_TASK_REPORT_MS = 5000;

/** Avvia il conteggio; restituisce la funzione che lo ferma. */
export function startLongTaskReporter(): () => void {
  if (!isTauri() || typeof PerformanceObserver === "undefined") return () => {};
  if (!PerformanceObserver.supportedEntryTypes?.includes("longtask")) return () => {};
  let count = 0;
  let max = 0;
  let total = 0;
  const observer = new PerformanceObserver((list) => {
    for (const e of list.getEntries()) {
      count += 1;
      total += e.duration;
      if (e.duration > max) max = e.duration;
    }
  });
  observer.observe({ type: "longtask", buffered: false });
  const timer = window.setInterval(() => {
    if (count === 0) return;
    void commands.reportLongTasks(count, max, total).catch(() => {});
    count = 0;
    max = 0;
    total = 0;
  }, LONG_TASK_REPORT_MS);
  return () => {
    observer.disconnect();
    window.clearInterval(timer);
  };
}
