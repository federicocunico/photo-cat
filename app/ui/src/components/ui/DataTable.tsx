import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

export type Column<R> = {
  key: string;
  header: ReactNode;
  cell: (row: R) => ReactNode;
  /** Larghezza CSS della colonna. */
  width?: string;
  align?: "start" | "end";
};

/**
 * Tabella con righe selezionabili da tastiera: ↑/↓, Home/Fine e Pagina su/giù spostano la
 * riga attiva (un solo tabstop), Invio o Spazio la aprono. Il clic fa lo stesso.
 */
export function DataTable<R>({
  label,
  columns,
  rows,
  rowId,
  selected,
  onSelect,
  onActivate,
  empty,
}: {
  /** Nome della tabella per il lettore di schermo. */
  label: string;
  columns: Column<R>[];
  rows: R[];
  rowId: (row: R) => string | number;
  /** Riga selezionata (controllata). */
  selected?: string | number | null;
  onSelect?: (row: R) => void;
  /** Invio/Spazio/doppio clic sulla riga. */
  onActivate?: (row: R) => void;
  empty?: ReactNode;
}) {
  const body = useRef<HTMLTableSectionElement>(null);
  const selIndex = rows.findIndex((r) => rowId(r) === selected);
  const [active, setActive] = useState(Math.max(0, selIndex));
  const focusOnMove = useRef(false);

  useEffect(() => {
    if (selIndex >= 0) setActive(selIndex);
  }, [selIndex]);
  useEffect(() => {
    if (active >= rows.length) setActive(Math.max(0, rows.length - 1));
  }, [rows.length, active]);
  useEffect(() => {
    if (!focusOnMove.current) return;
    focusOnMove.current = false;
    const tr = body.current?.querySelectorAll<HTMLTableRowElement>("tr")[active];
    tr?.focus();
    tr?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const move = (to: number) => {
    const next = Math.min(rows.length - 1, Math.max(0, to));
    focusOnMove.current = true;
    setActive(next);
    if (rows[next]) onSelect?.(rows[next]);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTableRowElement>, i: number) => {
    const page = 10;
    const keys: Record<string, () => void> = {
      ArrowDown: () => move(i + 1),
      ArrowUp: () => move(i - 1),
      Home: () => move(0),
      End: () => move(rows.length - 1),
      PageDown: () => move(i + page),
      PageUp: () => move(i - page),
      Enter: () => onActivate?.(rows[i]),
      " ": () => (onActivate ? onActivate(rows[i]) : onSelect?.(rows[i])),
    };
    const k = keys[e.key];
    if (!k || e.target !== e.currentTarget) return;
    e.preventDefault();
    k();
  };

  return (
    <table className="table ui-table" aria-label={label}>
      <thead>
        <tr>
          {columns.map((c) => (
            <th
              key={c.key}
              scope="col"
              style={{ width: c.width, textAlign: c.align === "end" ? "right" : undefined }}
            >
              {c.header}
            </th>
          ))}
        </tr>
      </thead>
      <tbody ref={body}>
        {rows.length === 0 && empty ? (
          <tr>
            <td colSpan={columns.length}>{empty}</td>
          </tr>
        ) : (
          rows.map((r, i) => {
            const id = rowId(r);
            const isSel = selected != null && id === selected;
            return (
              <tr
                key={id}
                tabIndex={i === active ? 0 : -1}
                aria-selected={onSelect ? isSel : undefined}
                data-selected={isSel || undefined}
                className="ui-table-row"
                onFocus={() => setActive(i)}
                onClick={() => {
                  setActive(i);
                  onSelect?.(r);
                }}
                onDoubleClick={() => onActivate?.(r)}
                onKeyDown={(e) => onKeyDown(e, i)}
              >
                {columns.map((c) => (
                  <td key={c.key} style={{ textAlign: c.align === "end" ? "right" : undefined }}>
                    {c.cell(r)}
                  </td>
                ))}
              </tr>
            );
          })
        )}
      </tbody>
    </table>
  );
}
