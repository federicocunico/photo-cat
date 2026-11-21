// Selezione di una griglia (design D1, github-issues-39-47): un insieme definito dal backend
// (`SelectionBase`: filtro o snapshot, più elementi aggiunti e tolti) che vale per qualunque
// vista con id numerici (foto, volti, sequenza). Ogni vista fornisce l'ordine visibile per
// Maiusc+clic e una funzione async che restituisce la base dell'insieme completo.
import { useCallback, useRef, useState } from "react";
import type { SelectionBase } from "./ipc";
import {
  clickSelect,
  idsSelection,
  invertOf,
  selCount,
  selectAllOf,
  selHas,
  selIds,
  selSet,
  type Selection,
} from "../store/library";

/** Dove vive la selezione: un archivio esterno (es. Libreria) oppure lo stato locale del hook. */
export interface GridSelectionState<F> {
  selection: Selection<F>;
  /** Selezione corrente al momento della chiamata (dopo un await). */
  getSelection: () => Selection<F>;
  /** Ultima ancora di Maiusc+clic, letta al momento del clic (può cambiare tra i render). */
  getAnchor: () => number | null;
  setAnchor: (id: number | null) => void;
  /** Sostituisce la selezione; `focused` undefined lascia invariato l'elemento con il focus. */
  replace: (selection: Selection<F>, focused?: number | null) => void;
}

export interface GridSelectionScope<F> {
  /** Filtro o riferimento che identifica l'insieme completo (copiato nella selezione). */
  filter: F;
  /** Id nell'ordine visibile (per l'intervallo di Maiusc+clic). */
  order: number[];
  /** Base dell'insieme completo, calcolata dal backend. */
  fetchBase: () => Promise<SelectionBase>;
  /** Archivio esterno; se assente il hook tiene la selezione in uno stato locale. */
  state?: GridSelectionState<F>;
  /** Errore del backend in Seleziona tutto / Inverti (predefinito: ignorato). */
  onError?: (e: unknown) => void;
}

export interface GridSelection<F> {
  selection: Selection<F>;
  count: number;
  has: (id: number) => boolean;
  /** Id espliciti, `null` se la selezione è per filtro. */
  ids: number[] | null;
  /** Clic: semplice, Ctrl/Cmd (aggiungi o togli), Maiusc (intervallo dall'ancora). */
  click: (id: number, mods: { ctrl: boolean; shift: boolean }) => void;
  /** Sostituisce con una selezione esplicita (rettangolo, tastiera). */
  setIds: (ids: number[], focused?: number | null) => void;
  replace: (selection: Selection<F>, focused?: number | null) => void;
  selectAll: () => Promise<void>;
  invert: () => Promise<void>;
  clear: () => void;
}

export function useGridSelection<F>(scope: GridSelectionScope<F>): GridSelection<F> {
  const [local, setLocal] = useState<{ selection: Selection<F>; anchor: number | null }>({
    selection: idsSelection<F>(),
    anchor: null,
  });
  const localRef = useRef(local);
  localRef.current = local;
  const latest = useRef(scope);
  latest.current = scope;

  const localState: GridSelectionState<F> = {
    selection: local.selection,
    getSelection: () => localRef.current.selection,
    getAnchor: () => localRef.current.anchor,
    setAnchor: (anchor) =>
      setLocal((l) => {
        localRef.current = { ...l, anchor };
        return localRef.current;
      }),
    replace: (selection) =>
      setLocal((l) => {
        localRef.current = { ...l, selection };
        return localRef.current;
      }),
  };
  const state = scope.state ?? localState;
  const stateRef = useRef(state);
  stateRef.current = state;
  const selection = state.selection;

  const replace = useCallback(
    (s: Selection<F>, focused?: number | null) => stateRef.current.replace(s, focused),
    [],
  );
  const setIds = useCallback((ids: number[], focused?: number | null) => {
    stateRef.current.replace(idsSelection<F>(ids), focused);
    if (!ids.length) stateRef.current.setAnchor(null);
  }, []);
  const click = useCallback((id: number, mods: { ctrl: boolean; shift: boolean }) => {
    const st = stateRef.current;
    const cur = st.getSelection();
    if (cur.mode === "filter" && mods.ctrl) {
      // "Tutti tranne…": Ctrl+clic aggiunge o toglie un'eccezione.
      st.replace(selSet(cur, [id], !selHas(cur, id)), id);
      st.setAnchor(id);
      return;
    }
    const anchor = st.getAnchor();
    const next = clickSelect(latest.current.order, selIds(cur) ?? [], anchor, id, mods);
    st.replace(idsSelection<F>(next), id);
    if (!next.length) st.setAnchor(null);
    // Maiusc non sposta l'ancora: solo clic semplice e Ctrl+clic.
    if (!mods.shift || anchor === null) st.setAnchor(id);
  }, []);
  const run = useCallback(async (apply: (base: SelectionBase) => void) => {
    try {
      apply(await latest.current.fetchBase());
    } catch (e) {
      latest.current.onError?.(e);
    }
  }, []);
  const selectAll = useCallback(
    () =>
      run((base) =>
        stateRef.current.replace(selectAllOf(latest.current.filter, base)),
      ),
    [run],
  );
  const invert = useCallback(
    () =>
      run((base) =>
        stateRef.current.replace(
          invertOf(stateRef.current.getSelection(), latest.current.filter, base),
        ),
      ),
    [run],
  );
  const clear = useCallback(() => setIds([]), [setIds]);

  return {
    selection,
    count: selCount(selection),
    has: (id) => selHas(selection, id),
    ids: selIds(selection),
    click,
    setIds,
    replace,
    selectAll,
    invert,
    clear,
  };
}
