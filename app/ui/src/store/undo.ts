import { create } from "zustand";
import { useCommands } from "../commands/registry";
import i18n from "../i18n";
import { notify, notifyError, useToast } from "./toasts";

/**
 * Registro globale dell'ultima operazione annullabile (design D6).
 *
 * Il backend tiene un solo registro LIFO (`undo_log`): il comando di annullamento
 * annulla sempre l'ultima voce. Per questo qui si ricorda una sola operazione, e ogni
 * comando che scrive nel registro del backend la invalida: Ctrl+Z non deve mai annullare
 * un'operazione diversa da quella mostrata.
 */
export type UndoEntry = {
  id: number;
  /** L'operazione in parole, ad esempio "Tag «mare» aggiunto a 12 foto". */
  label: string;
  /** Chiama il comando di annullamento già esistente e aggiorna le viste. */
  run: () => Promise<unknown>;
  /** Avviso con "Annulla operazione" legato a questa voce, chiuso quando la voce decade. */
  noticeId?: number;
};

type UndoState = { last: UndoEntry | null; busy: boolean };

export const useUndo = create<UndoState>(() => ({ last: null, busy: false }));

/**
 * Comandi che scrivono in `undo_log` (o lo consumano). Chiamarli fa decadere la voce
 * registrata, che subito dopo può essere sostituita da quella della nuova operazione.
 * Le operazioni senza annullamento lato backend sono elencate in `tasks.md` (1.5).
 */
export const UNDO_LOG_COMMANDS: ReadonlySet<string> = new Set([
  "startBulk",
  "renameTag",
  "mergeTag",
  "deleteTag",
  "correctPerson",
  "renamePlace",
  "deleteCategory",
  "setGroupShift",
  "placeUndated",
  "undoBulk",
  "undoPeople",
]);

let next = 0;

function drop(entry: UndoEntry | null) {
  if (entry?.noticeId != null) useToast.getState().dismiss(entry.noticeId);
}

/** Registra l'ultima operazione annullabile, al posto della precedente. */
export function registerUndo(input: { label: string; run: () => Promise<unknown> }): number {
  const entry: UndoEntry = { ...input, id: ++next };
  drop(useUndo.getState().last);
  useUndo.setState({ last: entry });
  return entry.id;
}

/**
 * Registra l'operazione e mostra l'avviso con "Annulla operazione". L'avviso e Ctrl+Z
 * chiamano lo stesso registro.
 */
export function notifyUndoable(input: {
  message: string;
  label?: string;
  run: () => Promise<unknown>;
  key?: string;
}): number {
  const id = registerUndo({ label: input.label ?? input.message, run: input.run });
  const noticeId = notify({
    key: input.key,
    message: input.message,
    tone: "ok",
    action: { label: i18n.t("common.undo"), run: () => runUndo(id) },
  });
  const last = useUndo.getState().last;
  if (last?.id === id) useUndo.setState({ last: { ...last, noticeId } });
  return id;
}

/** Fa decadere la voce (tutte, o solo quella con quell'id). */
export function clearUndo(id?: number) {
  const last = useUndo.getState().last;
  if (!last || (id != null && last.id !== id)) return;
  drop(last);
  useUndo.setState({ last: null });
}

/** Un comando del backend sta per scrivere in `undo_log`: la voce attuale non è più l'ultima. */
export function noteCommand(name: string) {
  if (UNDO_LOG_COMMANDS.has(name) && !useUndo.getState().busy) clearUndo();
}

/**
 * Annulla l'ultima operazione registrata (o quella con `id`, se è ancora l'ultima).
 * Restituisce `true` se ha annullato qualcosa.
 */
export async function runUndo(id?: number): Promise<boolean> {
  const { last, busy } = useUndo.getState();
  if (busy || !last || (id != null && last.id !== id)) return false;
  drop(last);
  useUndo.setState({ last: null, busy: true });
  try {
    await last.run();
    notify({ key: "undo", message: i18n.t("common.undone", { label: last.label }) });
    return true;
  } catch (error) {
    notifyError(error);
    return false;
  } finally {
    useUndo.setState({ busy: false });
  }
}

/**
 * Ctrl+Z (⌘Z) globale: annulla l'ultima operazione registrata, da qualunque modulo.
 * Nei campi di testo resta l'annullamento del testo; senza voce registrata il tasto
 * passa ai gestori dei moduli (ad esempio Persone).
 */
export function useUndoHotkey() {
  useCommands([
    {
      id: "undo",
      label: i18n.t("commands.undo"),
      group: "general",
      shortcut: { key: "z", mod: true },
      when: () => useUndo.getState().last != null,
      run: () => runUndo(),
    },
  ]);
}
