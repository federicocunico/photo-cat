// Bersaglio della pulizia: tutta la libreria o un catalogo, con `maxId` dalla selezione di base.
import { commands, type Target } from "../../../lib/ipc";
import { emptyFilter } from "../../../store/library";

export async function scopeTarget(catalogId: number | null): Promise<Target> {
  const filter = { ...emptyFilter(), ...(catalogId != null ? { catalogId } : {}) };
  const base = await commands.librarySelection(filter);
  return { kind: "filter", filter, maxId: base.maxId, except: [] };
}
