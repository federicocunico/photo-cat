import { commands } from "../../lib/ipc";
import { useGridSelection, type GridSelectionState } from "../../lib/useGridSelection";
import { useLibrary } from "../../store/library";
import { notifyError } from "../../store/toasts";

/** La selezione della Libreria e dei cataloghi vive nello store `useLibrary`. */
const libraryState = (selection: ReturnType<typeof useLibrary.getState>["selection"]) =>
  ({
    selection,
    getSelection: () => useLibrary.getState().selection,
    getAnchor: () => useLibrary.getState().anchor,
    setAnchor: (id) => useLibrary.getState().setAnchor(id),
    replace: (s, focused) => useLibrary.getState().replaceSelection(s, focused),
  }) satisfies GridSelectionState<ReturnType<typeof useLibrary.getState>["filter"]>;

/** `useGridSelection` sulla Libreria; `order` serve a Maiusc+clic (assente: nessun intervallo). */
export function useLibrarySelection(order: number[] = []) {
  const filter = useLibrary((s) => s.filter);
  const selection = useLibrary((s) => s.selection);
  return useGridSelection({
    filter,
    order,
    fetchBase: () => commands.librarySelection(filter),
    state: libraryState(selection),
    onError: notifyError,
  });
}
