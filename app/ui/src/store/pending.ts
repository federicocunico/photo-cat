import { create } from "zustand";
import i18n from "../i18n";
import { notify } from "./toasts";

export const usePending = create<{ count: number }>(() => ({ count: 0 }));
const polling = new Set([
  "listJobs",
  "listJobHistory",
  "getResources",
  "logTail",
  "queryErrors",
  "listModels",
  "bootStatus",
]);
const mutation =
  /^(save|merge|rename|delete|set|correct|add|remove|undo|link|complete|recompute|purge|importIdentities)/;

/** Track the actual IPC promise, including failures and commands outside queries. */
export async function trackCommand<T>(name: string, run: () => Promise<T>): Promise<T> {
  const tracked = !polling.has(name);
  if (tracked) usePending.setState((s) => ({ count: s.count + 1 }));
  const started = Date.now();
  try {
    // L'esito positivo lo annuncia chi chiama il comando, con un testo che dice che cosa è stato
    // fatto e su che cosa (ui-foundations, "Avvisi non modali"): qui nessun avviso generico.
    return await run();
  } catch (error) {
    if (mutation.test(name) && Date.now() - started >= 1000)
      notify({ message: i18n.t("feedback.failed"), tone: "error" });
    throw error;
  } finally {
    if (tracked) usePending.setState((s) => ({ count: s.count - 1 }));
  }
}
