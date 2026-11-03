import type { QueryClient } from "@tanstack/react-query";
import { commands } from "../../lib/ipc";
import { notifyUndoable } from "../../store/undo";

/** Aggiorna elenco, conteggi, volti e identità dopo una correzione o un annullamento. */
export function refreshPeople(qc: QueryClient) {
  return Promise.all([
    qc.invalidateQueries({ queryKey: ["people-sections"] }),
    qc.invalidateQueries({ queryKey: ["people-groups"] }),
    qc.invalidateQueries({ queryKey: ["group-faces"] }),
    qc.invalidateQueries({ queryKey: ["people"] }),
    qc.invalidateQueries({ queryKey: ["identities"] }),
    qc.invalidateQueries({ queryKey: ["people-move-targets"] }),
    qc.invalidateQueries({ queryKey: ["faces"] }),
  ]);
}

/**
 * Dopo una correzione (issue #31): aspetta solo i volti mostrati, che è ciò che l'utente
 * vede cambiare; elenco, conteggi e identità si aggiornano in background. Su librerie
 * grandi quelle query durano secondi e bloccavano i comandi fino alla fine.
 */
export function refreshPeopleFast(qc: QueryClient) {
  const faces = qc.invalidateQueries({ queryKey: ["group-faces"] });
  for (const key of [
    "people-sections",
    "people-groups",
    "people",
    "identities",
    "faces",
    "people-move-targets",
  ])
    void qc.invalidateQueries({ queryKey: [key] });
  return faces;
}

/**
 * Registra una correzione delle persone nello store `undo`: avviso con "Annulla operazione"
 * e Ctrl+Z. `steps` è il numero di voci scritte nel registro del backend (ad esempio una
 * per volto in "Non è questa persona"): l'annullamento le toglie tutte, dall'ultima.
 */
export function notifyPeopleUndo(qc: QueryClient, message: string, steps = 1) {
  return notifyUndoable({
    key: "people-undo",
    message,
    run: async () => {
      try {
        for (let i = 0; i < steps; i++) await commands.undoPeople();
      } finally {
        await refreshPeopleFast(qc);
      }
    },
  });
}
