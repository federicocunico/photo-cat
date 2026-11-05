// Navigazione dalla sezione Risultati verso la Libreria e la sezione Pulizia.
import type { LibraryFilter } from "../../../lib/ipc";
import { useLayout } from "../../../store/layout";
import { emptyFilter, useLibrary } from "../../../store/library";
import { useCleanupPreset } from "./cleanupPreset";

/** Apre la Libreria con un filtro nuovo (ambito più condizioni), azzerando la selezione. */
export function openLibraryWith(scopeCatalogId: number | null, extra: Partial<LibraryFilter>) {
  const lib = useLibrary.getState();
  lib.setSelection([], null);
  lib.setFilter({ ...emptyFilter(), catalogId: scopeCatalogId, ...extra });
  useLayout.getState().openModule({ module: "library" });
}

/** Apre la sezione Pulizia con la voce e il limite di confidenza già impostati. */
export function openCleanupWith(
  scopeCatalogId: number | null,
  grp: string,
  value: string,
  maxScore: number | null,
) {
  useCleanupPreset.getState().set({ grp, value, maxScore, scopeCatalogId });
  useLayout.getState().openModule({ module: "ai", section: "cleanup" });
}
