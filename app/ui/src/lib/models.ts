import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { commands, events, isTauri, type ModelStatus } from "./ipc";

/** Stato dei modelli AI: elenco iniziale più gli aggiornamenti in tempo reale. */
export function useModelStatuses(): ModelStatus[] {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const list = useQuery({
    queryKey: ["models"],
    queryFn: () => commands.listModels(),
    enabled: isTauri(),
  });
  useEffect(() => {
    if (list.data) setModels(list.data);
  }, [list.data]);
  useEffect(() => {
    const un = events.onModels(setModels);
    return () => {
      void un.then((f) => f());
    };
  }, []);
  return models;
}

export type ModelsSummary =
  | { kind: "none" }
  | { kind: "ready" }
  | { kind: "missing" }
  | { kind: "update" }
  | { kind: "error" }
  | { kind: "downloading"; percent: number };

/** Un solo stato per la barra: il più urgente tra quelli dei modelli. */
export function summarizeModels(models: ModelStatus[]): ModelsSummary {
  if (models.length === 0) return { kind: "none" };
  const states = models.map((m) => m.state);
  if (states.some((s) => s.state === "error" || s.state === "self_test_failed"))
    return { kind: "error" };
  const downloading = states.filter((s) => s.state === "downloading");
  if (downloading.length) {
    let done = 0;
    let total = 0;
    for (const s of downloading)
      if (s.state === "downloading") {
        done += s.done_bytes;
        total += s.total_bytes;
      }
    return { kind: "downloading", percent: Math.floor((done / Math.max(1, total)) * 100) };
  }
  if (states.some((s) => s.state === "missing")) return { kind: "missing" };
  if (states.some((s) => s.state === "update_available")) return { kind: "update" };
  return { kind: "ready" };
}
