// Dati dei tag condivisi da Tag, Libreria e Sequenza: riepilogo, set attivo, colori.
import { useQuery, type QueryClient } from "@tanstack/react-query";
import { commands, isTauri, type TagOverview, type TagSetView } from "../../lib/ipc";

export const TAGS_KEY = ["tags"] as const;

export function useTagOverview() {
  return useQuery({ queryKey: TAGS_KEY, queryFn: commands.tagOverview, enabled: isTauri() });
}

export function activeSet(o: TagOverview | undefined): TagSetView | undefined {
  if (!o) return undefined;
  return o.sets.find((s) => s.id === o.activeSet) ?? o.sets[0];
}

/** Colore (chiave della palette) di un tag, dal riepilogo. */
export function colorOf(o: TagOverview | undefined, name: string): string {
  const lower = name.toLowerCase();
  return o?.tags.find((t) => t.name.toLowerCase() === lower)?.color ?? "gray";
}

/** Dopo una modifica a tag o set: aggiorna riepilogo, Libreria e "Annulla". */
export async function refreshTags(qc: QueryClient) {
  await Promise.all([
    qc.invalidateQueries({ queryKey: TAGS_KEY }),
    qc.invalidateQueries({ queryKey: ["library"] }),
    qc.invalidateQueries({ queryKey: ["library-count"] }),
    qc.invalidateQueries({ queryKey: ["media-detail"] }),
    qc.invalidateQueries({ queryKey: ["bulk-undo"] }),
    qc.invalidateQueries({ queryKey: ["sequence"] }),
    qc.invalidateQueries({ queryKey: ["sequence-items"] }),
  ]);
}

/** Testo dei tasti di un set ("1–5", "1–3, 5"). */
export function keyRange(keys: number[]): string {
  const sorted = [...keys].sort((a, b) => a - b);
  const parts: string[] = [];
  for (let i = 0; i < sorted.length;) {
    let j = i;
    while (j + 1 < sorted.length && sorted[j + 1] === sorted[j] + 1) j++;
    parts.push(j > i ? `${sorted[i]}–${sorted[j]}` : `${sorted[i]}`);
    i = j + 1;
  }
  return parts.join(", ");
}

export const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
