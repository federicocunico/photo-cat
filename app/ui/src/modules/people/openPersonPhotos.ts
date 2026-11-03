import { commands } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";

/**
 * Tutte le foto di una persona (issue #23): Libreria filtrata sulla persona, con il
 * chip del filtro, e tutte le sue foto selezionate per le azioni sulla selezione.
 */
export async function openPersonPhotos(person: number) {
  const lib = useLibrary.getState();
  lib.setFilter({ ...emptyFilter(), personIds: [person] });
  const base = await commands.librarySelection(useLibrary.getState().filter);
  useLibrary.getState().selectAll(base);
  useLayout.getState().openModule({ module: "library" });
}
