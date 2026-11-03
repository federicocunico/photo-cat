// Righe di una lista di filtro (casella, nome, numero di foto, azione
// facoltativa) e l'elenco completo virtualizzato, caricato a pagine.
import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { FacetValue } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";

export type RowAction = (value: FacetValue) => ReactNode;

export function FacetRow({
  value,
  checked,
  onToggle,
  render,
  action,
}: {
  value: FacetValue;
  checked: boolean;
  onToggle: (v: string) => void;
  render: (v: string) => string;
  action?: RowAction;
}) {
  const name = render(value.value);
  return (
    <div className="facet-row">
      <label className="facet-row-label">
        <input type="checkbox" checked={checked} onChange={() => onToggle(value.value)} />
        <span title={name} className="facet-row-name" data-checked={checked || undefined}>
          {name}
        </span>
        <span className="muted facet-row-count">{fmtInt(value.count)}</span>
      </label>
      {action?.(value)}
    </div>
  );
}

const ROW = 26;

export function FacetValuesList({
  values,
  hasMore,
  loadMore,
  loading,
  selected,
  onToggle,
  render,
  action,
  height,
  fill,
}: {
  values: FacetValue[];
  hasMore: boolean;
  loadMore: () => void;
  loading: boolean;
  selected: string[];
  onToggle: (v: string) => void;
  render: (v: string) => string;
  action?: RowAction;
  /** Altezza massima (lista nella barra laterale). */
  height?: number;
  /** Occupa tutto lo spazio disponibile (pannello flottante). */
  fill?: boolean;
}) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const count = values.length + (hasMore ? 1 : 0);
  const virt = useVirtualizer({
    count,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW,
    overscan: 8,
    initialRect: { width: 280, height: height ?? 360 },
  });
  const items = virt.getVirtualItems();
  const last = items[items.length - 1]?.index ?? -1;
  // Scorrendo verso il fondo si carica la pagina successiva.
  useEffect(() => {
    if (hasMore && last >= values.length - 1) loadMore();
  }, [last, values.length, hasMore, loadMore]);
  if (!loading && values.length === 0)
    return <span className="muted">{t("library.facets.noMatch")}</span>;
  return (
    <div
      ref={scrollRef}
      role="group"
      className="facet-values"
      data-fill={fill || undefined}
      style={{ maxHeight: fill ? undefined : (height ?? 360) }}
    >
      <div className="facet-values-inner" style={{ height: virt.getTotalSize() }}>
        {items.map((it) => (
          <div
            key={it.key}
            className="facet-values-item"
            style={{ transform: `translateY(${it.start}px)` }}
          >
            {it.index < values.length ? (
              <FacetRow
                value={values[it.index]}
                checked={selected.includes(values[it.index].value)}
                onToggle={onToggle}
                render={render}
                action={action}
              />
            ) : (
              <span className="muted">{t("library.facets.loading")}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
