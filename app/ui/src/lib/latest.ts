// Richieste "vince l'ultima": conteggi e facet di un filtro già cambiato non servono più.
// Ogni chiamata riceve un numero crescente per canale; il backend scarta o interrompe le
// richieste più vecchie (risposta `null`) e qui si scarta anche una risposta arrivata tardi.

import { useQuery } from "@tanstack/react-query";
import { commands, isTauri, type LibraryFilter, type RequestTag } from "./ipc";

const counters = new Map<string, number>();

/** Risposta di una richiesta superata da una più recente dello stesso canale. */
export class Superseded extends Error {
  constructor() {
    super("richiesta superata");
  }
}

export const isSuperseded = (e: unknown) => e instanceof Superseded;

/** Esegue `call` come ultima richiesta del canale; lancia `Superseded` se nel frattempo ne è partita un'altra. */
export async function latestOnly<T>(
  channel: string,
  call: (tag: RequestTag) => Promise<T | null>,
): Promise<T> {
  const id = (counters.get(channel) ?? 0) + 1;
  counters.set(channel, id);
  const result = await call({ channel, id });
  if (result === null || counters.get(channel) !== id) throw new Superseded();
  return result;
}

/** Una richiesta superata non va ripetuta: ne è già partita una più recente. */
export const retryUnlessSuperseded = (failures: number, error: unknown) =>
  !isSuperseded(error) && failures < 1;

/** Conteggio del filtro (griglia e barra di stato condividono la stessa query). */
export function useLibraryCount(filter: LibraryFilter) {
  return useQuery({
    queryKey: ["library-count", filter],
    queryFn: () => latestOnly("library-count", (tag) => commands.libraryCount(filter, tag)),
    enabled: isTauri(),
    retry: retryUnlessSuperseded,
    placeholderData: (prev) => prev,
  });
}
