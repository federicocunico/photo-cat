import { fmtInt } from "../../lib/format";
import { useMemo } from "react";
import { activateOnKey, EmptyState, truncate, useTip, useWidth } from "./shared";

export interface BarItem {
  key: string;
  label: string;
  value: number;
}
export interface BarChartProps {
  items: BarItem[];
  onSelect?: (key: string) => void;
  /** Ordina dal valore più alto (default true). */
  sorted?: boolean;
  emptyText: string;
  /** Etichetta accessibile del grafico. */
  ariaLabel: string;
  /** Etichetta accessibile di una barra; default "etichetta: valore". */
  barLabel?: (item: BarItem) => string;
  formatValue?: (n: number) => string;
}

const ROW = 28;
const BAR = 14;

export function BarChart({
  items,
  onSelect,
  sorted = true,
  emptyText,
  ariaLabel,
  barLabel,
  formatValue,
}: BarChartProps) {
  const { ref, width } = useWidth();
  const { show, hide, node } = useTip(width);
  const rows = useMemo(
    () => (sorted ? [...items].sort((a, b) => b.value - a.value) : items),
    [items, sorted],
  );
  const fmt = formatValue ?? fmtInt;
  const labelW = Math.min(170, Math.round(width * 0.38));
  const valueW = 56;
  const plotW = Math.max(20, width - labelW - valueW - 8);
  const max = Math.max(1, ...rows.map((r) => r.value));

  return (
    <div className="pc-chart" ref={ref}>
      {rows.length === 0 ? (
        <EmptyState text={emptyText} />
      ) : (
        <svg width={width} height={rows.length * ROW} role="group" aria-label={ariaLabel}>
          {rows.map((r, i) => {
            const y = i * ROW;
            const w = r.value > 0 ? Math.max(2, (r.value / max) * plotW) : 0;
            const label = barLabel ? barLabel(r) : `${r.label}: ${fmt(r.value)}`;
            const tipAt = () =>
              show(
                labelW + 8 + w / 2,
                y,
                <>
                  {r.label}: <strong>{fmt(r.value)}</strong>
                </>,
              );
            return (
              <g
                key={r.key}
                className="pc-chart-mark"
                data-testid={`bar-${r.key}`}
                role={onSelect ? "button" : "img"}
                tabIndex={onSelect ? 0 : undefined}
                aria-label={label}
                onClick={onSelect ? () => onSelect(r.key) : undefined}
                onKeyDown={onSelect ? activateOnKey(() => onSelect(r.key)) : undefined}
                onPointerEnter={tipAt}
                onPointerLeave={hide}
                onFocus={tipAt}
                onBlur={hide}
              >
                <rect
                  className="pc-chart-ring"
                  x={0}
                  y={y + 1}
                  width={width}
                  height={ROW - 2}
                  rx={4}
                />
                <rect className="pc-chart-hit" x={0} y={y} width={width} height={ROW} />
                <text x={labelW} y={y + ROW / 2 + 4} textAnchor="end">
                  {truncate(r.label, Math.floor(labelW / 6.5))}
                </text>
                <rect
                  className="pc-chart-fill"
                  x={labelW + 8}
                  y={y + (ROW - BAR) / 2}
                  width={w}
                  height={BAR}
                  rx={4}
                />
                <text className="pc-chart-value" x={labelW + 8 + w + 6} y={y + ROW / 2 + 4}>
                  {fmt(r.value)}
                </text>
              </g>
            );
          })}
        </svg>
      )}
      {node}
    </div>
  );
}
