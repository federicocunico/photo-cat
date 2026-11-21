import { commands, type SequenceScope, type Target } from "../../lib/ipc";
import { selTarget, type Selection } from "../../store/library";

/**
 * Bersaglio delle azioni bulk: id espliciti, oppure (dopo Seleziona tutto) gli id risolti dal
 * backend come ambito meno eccezioni, limitati alle foto presenti al momento della selezione.
 */
export async function sequenceTarget(
  scope: SequenceScope,
  selection: Selection<SequenceScope>,
): Promise<Target> {
  if (selection.mode === "ids") return selTarget(selection as Selection);
  const ids = await commands.sequenceSelectionIds(scope, selection.maxId, [...selection.except]);
  return { kind: "ids", ids };
}
