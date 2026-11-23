// Raccolta di prova per i test del modello di selezione (unified-selection): usa gli stessi hook
// delle raccolte vere (useSelection, useSelectionKeys, selectForContextMenu, annunci) con un
// markup minimo.
import { useState, type MouseEvent } from "react";
import { selectForContextMenu, selCount } from "../lib/selection/model";
import { useSelection } from "../lib/selection/useSelection";
import { useSelectionKeys } from "../lib/selection/useSelectionKeys";
import { useSelectionAnnouncer } from "../lib/selection/announce";
import { SelectionLiveRegion } from "../lib/selection/SelectionLiveRegion";

export function SelectionHarness({
  ids,
  layout,
  columns = 4,
  pageSize = 8,
  withInput = false,
}: {
  ids: number[];
  layout: "grid" | "list";
  columns?: number;
  pageSize?: number;
  /** Un campo di testo dentro la raccolta (es. rinomina sul posto). */
  withInput?: boolean;
}) {
  const sel = useSelection<null>({ filter: null, order: ids });
  const [cursor, setCursor] = useState(-1);
  const [menuFor, setMenuFor] = useState<number | null>(null);
  useSelectionAnnouncer(sel.count, "photos");
  const onKeyDown = useSelectionKeys<number>({
    layout,
    columns,
    pageSize,
    count: ids.length,
    cursor,
    idAt: (i) => ids[i],
    moveTo: setCursor,
    select: sel.click,
    ensureAnchor: sel.ensureAnchor,
    selectAll: () => void sel.selectAll(),
    invert: () => void sel.invert(),
    clear: sel.clear,
  });
  const itemRole = layout === "grid" ? "gridcell" : "option";
  const onItemClick = (i: number) => (e: MouseEvent) => {
    e.stopPropagation();
    setCursor(i);
    sel.click(ids[i]!, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
  };
  const onItemMenu = (i: number) => (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const id = ids[i]!;
    const next = selectForContextMenu(sel.selection, id);
    if (next !== sel.selection) sel.replace(next, id);
    setCursor(i);
    setMenuFor(selCount(next));
  };
  return (
    <>
      <div
        role={layout === "grid" ? "grid" : "listbox"}
        aria-multiselectable
        aria-label="raccolta"
        tabIndex={0}
        aria-activedescendant={cursor >= 0 ? `item-${ids[cursor]}` : undefined}
        onKeyDown={onKeyDown}
        onClick={() => sel.clear()}
      >
        {withInput && <input aria-label="rinomina" defaultValue="" />}
        {ids.map((id, i) => (
          <div
            key={id}
            id={`item-${id}`}
            role={itemRole}
            data-id={id}
            aria-selected={sel.has(id)}
            onClick={onItemClick(i)}
            onContextMenu={onItemMenu(i)}
          >
            {id}
          </div>
        ))}
      </div>
      {menuFor != null && <div role="menu" aria-label={`Azioni su ${menuFor} elementi`} />}
      <SelectionLiveRegion />
    </>
  );
}
