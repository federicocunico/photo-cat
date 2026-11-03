// Cataloghi di sistema (change system-catalogs): Cestino, Duplicati, Qualità insufficiente.
import type { TFunction } from "i18next";
import type { QueryClient } from "@tanstack/react-query";
import { commands, type Catalog, type CatalogSummary } from "../../lib/ipc";
import { notify, notifyError } from "../../store/toasts";
import { notifyUndoable } from "../../store/undo";

export type SystemRole = "trash" | "duplicates" | "low_quality";

/** Nome del catalogo nella lingua dell'interfaccia (quelli di sistema per ruolo). */
export function catalogLabel(c: Pick<Catalog, "name" | "system">, t: TFunction): string {
  return c.system ? t(`catalogs.system.${c.system}`) : c.name;
}

/** Id del catalogo di sistema con quel ruolo, se l'elenco è caricato. */
export function systemCatalogId(list: CatalogSummary[] | undefined, role: SystemRole) {
  return list?.find((s) => s.catalog.system === role)?.catalog.id;
}

/** "Metti da parte i duplicati" (issue #29), con "Annulla operazione". */
export async function collectDuplicates(qc: QueryClient, t: TFunction) {
  try {
    const n = await commands.collectDuplicates();
    await qc.invalidateQueries();
    if (n === 0) {
      notify({ message: t("catalogs.system.noDuplicates") });
      return;
    }
    notifyUndoable({
      message: t("catalogs.system.duplicatesDone", { count: n }),
      run: async () => {
        await commands.undoBulk();
        await qc.invalidateQueries();
      },
    });
  } catch (error) {
    notifyError(error);
  }
}

/** "Controlla la qualità" (issue #20): attività con avanzamento in Attività. */
export async function startQualityCheck(t: TFunction) {
  try {
    await commands.startPhotoQuality();
    notify({ message: t("catalogs.system.qualityStarted") });
  } catch (error) {
    notifyError(error);
  }
}
