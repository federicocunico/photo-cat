// Selezione di una raccolta (unified-selection, design D1). La forma è quella del modello comune:
// id espliciti, oppure un insieme definito dal backend (filtro o snapshot) più aggiunte e tolte.
// Ogni raccolta fornisce l'ordine visibile (per Maiusc) e il modo di ottenere "tutti gli elementi":
// - `fetchBase`: base del backend per una selezione per filtro (id numerici, raccolte grandi);
// - `fetchAll`: elenco esplicito degli id (raccolte piccole o con id testuali, es. le identità).
import { useCallback, useRef, useState } from "react";
import type { SelectionBase } from "../ipc";
import {
  clickSelect,
  idsSelection,
  invertOf,
  selCount,
  selectAllOf,
  selHas,
  selIds,
  selSet,
  type Key,
  type Selection,
} from "./model";

/** Dove vive la selezione: un archivio esterno (es. Libreria) oppure lo stato locale del hook. */
export interface SelectionStore<F, Id extends Key = number> {
  selection: Selection<F, Id>;
  /** Selezione corrente al momento della chiamata (dopo un await). */
  getSelection: () => Selection<F, Id>;
  /** Ultima ancora degli intervalli, letta al momento del gesto (può cambiare tra i render). */
  getAnchor: () => Id | null;
  setAnchor: (id: Id | null) => void;
  /** Sostituisce la selezione; `focused` undefined lascia invariato l'elemento con il focus. */
  replace: (selection: Selection<F, Id>, focused?: Id | null) => void;
}

export interface SelectionScope<F, Id extends Key = number> {
  /** Filtro o riferimento che identifica l'insieme completo (copiato nella selezione). */
  filter: F;
  /** Id nell'ordine visibile (per gli intervalli con Maiusc). */
  order: readonly Id[];
  /** Base dell'insieme completo, calcolata dal backend (selezione per filtro). */
  fetchBase?: () => Promise<SelectionBase>;
  /** Tutti gli id della raccolta, se la raccolta non usa la selezione per filtro. */
  fetchAll?: () => Promise<Id[]>;
  /** Archivio esterno; se assente il hook tiene la selezione in uno stato locale. */
  state?: SelectionStore<F, Id>;
  /** Errore del backend in Seleziona tutto / Inverti (predefinito: ignorato). */
  onError?: (e: unknown) => void;
}

export interface SelectionHandle<F, Id extends Key = number> {
  selection: Selection<F, Id>;
  count: number;
  has: (id: Id) => boolean;
  /** Id espliciti, `null` se la selezione è per filtro. */
  ids: Id[] | null;
  /** Ancora corrente degli intervalli. */
  anchor: () => Id | null;
  /** Usa `id` come ancora solo se non ce n'è già una (Maiusc+freccia dal focus). */
  ensureAnchor: (id: Id) => void;
  /** Gesto su un elemento: semplice, Ctrl/Cmd (aggiungi o togli), Maiusc (intervallo dall'ancora). */
  click: (id: Id, mods: { ctrl: boolean; shift: boolean }) => void;
  /** Sostituisce con una selezione esplicita (rettangolo, tastiera). */
  setIds: (ids: Id[], focused?: Id | null) => void;
  replace: (selection: Selection<F, Id>, focused?: Id | null) => void;
  selectAll: () => Promise<void>;
  invert: () => Promise<void>;
  clear: () => void;
}

export function useSelection<F, Id extends Key = number>(
  scope: SelectionScope<F, Id>,
): SelectionHandle<F, Id> {
  const [local, setLocal] = useState<{ selection: Selection<F, Id>; anchor: Id | null }>({
    selection: idsSelection<F, Id>(),
    anchor: null,
  });
  const localRef = useRef(local);
  localRef.current = local;
  const latest = useRef(scope);
  latest.current = scope;

  const localState: SelectionStore<F, Id> = {
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
    (s: Selection<F, Id>, focused?: Id | null) => stateRef.current.replace(s, focused),
    [],
  );
  const setIds = useCallback((ids: Id[], focused?: Id | null) => {
    stateRef.current.replace(idsSelection<F, Id>(ids), focused);
    if (!ids.length) stateRef.current.setAnchor(null);
  }, []);
  const click = useCallback((id: Id, mods: { ctrl: boolean; shift: boolean }) => {
    const st = stateRef.current;
    const cur = st.getSelection();
    const anchor = st.getAnchor();
    if (cur.mode === "filter" && mods.ctrl) {
      if (mods.shift && anchor !== null) {
        // Ctrl+Maiusc: l'intervallo si aggiunge all'insieme "tutti tranne…".
        const range = clickSelect(latest.current.order, [], anchor, id, {
          ctrl: false,
          shift: true,
        });
        st.replace(selSet(cur, range, true), id);
        return;
      }
      // "Tutti tranne…": Ctrl+clic aggiunge o toglie un'eccezione.
      st.replace(selSet(cur, [id], !selHas(cur, id)), id);
      st.setAnchor(id);
      return;
    }
    const next = clickSelect(latest.current.order, selIds(cur) ?? [], anchor, id, mods);
    st.replace(idsSelection<F, Id>(next), id);
    if (!next.length) st.setAnchor(null);
    // Maiusc non sposta l'ancora: solo clic semplice e Ctrl+clic.
    if (!mods.shift || anchor === null) st.setAnchor(id);
  }, []);
  const selectAll = useCallback(async () => {
    const sc = latest.current;
    try {
      if (sc.fetchBase) {
        const base = await sc.fetchBase();
        stateRef.current.replace(selectAllOf<F, Id>(latest.current.filter, base));
      } else if (sc.fetchAll) {
        stateRef.current.replace(idsSelection<F, Id>(await sc.fetchAll()));
      } else {
        stateRef.current.replace(idsSelection<F, Id>(sc.order));
      }
    } catch (e) {
      latest.current.onError?.(e);
    }
  }, []);
  const invert = useCallback(async () => {
    const sc = latest.current;
    try {
      if (sc.fetchBase) {
        const base = await sc.fetchBase();
        stateRef.current.replace(
          invertOf<F, Id>(stateRef.current.getSelection(), latest.current.filter, base),
        );
        return;
      }
      const all = sc.fetchAll ? await sc.fetchAll() : [...sc.order];
      const cur = stateRef.current.getSelection();
      stateRef.current.replace(idsSelection<F, Id>(all.filter((id) => !selHas(cur, id))));
    } catch (e) {
      latest.current.onError?.(e);
    }
  }, []);
  const clear = useCallback(() => setIds([]), [setIds]);
  const anchor = useCallback(() => stateRef.current.getAnchor(), []);
  const ensureAnchor = useCallback((id: Id) => {
    if (stateRef.current.getAnchor() === null) stateRef.current.setAnchor(id);
  }, []);

  return {
    selection,
    count: selCount(selection),
    has: (id) => selHas(selection, id),
    ids: selIds(selection),
    anchor,
    ensureAnchor,
    click,
    setIds,
    replace,
    selectAll,
    invert,
    clear,
  };
}
