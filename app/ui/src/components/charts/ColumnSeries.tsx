import { fmtInt } from "../../lib/format";
import { activateOnKey, EmptyState, useTip, useWidth } from "./shared";

export interface ColumnItem {
  key: string;
  label: string;
  total: number;
  analyzed: number;
}
export interface ColumnSeriesProps {
  items: ColumnItem[];
  /** Colonna finale "senza data", separata dalle altre. */
  noDate?: ColumnItem;
  onSelect?: (key: string) => void;
  emptyText: string;
  ariaLabel: string;
  totalLabel: string;
  analyzedLabel: string;
  formatValue?: (n: number) => string;
}

const H = 180;
const TOP = 8;
const BOTTOM = 22;
const LEFT = 40;

export function ColumnSeries({
  items,
  noDate,
  onSelect,
  emptyText,
  ariaLabel,
  totalLabel,
  analyzedLabel,
  formatValue,
}: ColumnSeriesProps) {
  const { ref, width } = useWidth();
  const { show, hide, node } = useTip(width);
  const fmt = formatValue ?? fmtInt;
  const cols = noDate ? [...items, noDate] : items;
  const max = Math.max(1, ...cols.map((c) => c.total));
  const plotW = Math.max(40, width - LEFT);
  const plotH = H - TOP - BOTTOM;
  const slot = plotW / Math.max(1, cols.length);
  const colW = Math.max(2, Math.min(28, slot - 2));
  const step = Math.max(1, Math.ceil(54 / slot));
  const y = (v: number) => TOP + plotH - (v / max) * plotH;
  const empty = cols.length === 0 || cols.every((c) => c.total === 0);
  const last = cols.length - 1;

  return (
    <div className="pc-chart" ref={ref}>
      {empty ? (
        <EmptyState text={emptyText} />
      ) : (
        <>
          <svg width={width} height={H} role="group" aria-label={ariaLabel}>
            {[0, 0.5, 1].map((f) => (
              <g key={f}>
                <line
                  className="pc-chart-grid"
                  x1={LEFT}
                  x2={width}
                  y1={y(max * f)}
                  y2={y(max * f)}
                />
                <text x={LEFT - 6} y={y(max * f) + 4} textAnchor="end">
                  {fmt(Math.round(max * f))}
                </text>
              </g>
            ))}
            {cols.map((c, i) => {
              const cx = LEFT + i * slot + (slot - colW) / 2;
              const isNoDate = noDate !== undefined && i === last;
              // Etichette diradate; quella "senza data" ha la precedenza sulle vicine.
              const crowded = noDate !== undefined && !isNoDate && last - i < step;
              const showLabel = isNoDate || (i % step === 0 && !crowded);
              const label = `${c.label}: ${totalLabel} ${fmt(c.total)}, ${analyzedLabel} ${fmt(c.analyzed)}`;
              const tipAt = () =>
                show(
                  cx + colW / 2,
                  y(c.total),
                  <>
                    <div>{c.label}</div>
                    <div>
                      {totalLabel}: <strong>{fmt(c.total)}</strong>
                    </div>
                    <div>
                      {analyzedLabel}: <strong>{fmt(c.analyzed)}</strong>
                    </div>
                  </>,
                );
              const analyzed = Math.min(c.analyzed, c.total);
              return (
                <g
                  key={c.key}
                  className="pc-chart-mark"
                  data-testid={`column-${c.key}`}
                  role={onSelect ? "button" : "img"}
                  tabIndex={onSelect ? 0 : undefined}
                  aria-label={label}
                  onClick={onSelect ? () => onSelect(c.key) : undefined}
                  onKeyDown={onSelect ? activateOnKey(() => onSelect(c.key)) : undefined}
                  onPointerEnter={tipAt}
                  onPointerLeave={hide}
                  onFocus={tipAt}
                  onBlur={hide}
                >
                  <rect
                    className="pc-chart-hit"
                    x={LEFT + i * slot}
                    y={TOP}
                    width={slot}
                    height={plotH + BOTTOM}
                  />
                  <rect
                    className="pc-chart-fill-soft"
                    x={cx}
                    y={y(c.total)}
                    width={colW}
                    height={TOP + plotH - y(c.total)}
                    rx={3}
                  />
                  {analyzed > 0 && (
                    <rect
                      className="pc-chart-fill"
                      x={cx}
                      y={y(analyzed)}
                      width={colW}
                      height={TOP + plotH - y(analyzed)}
                      rx={3}
                    />
                  )}
                  <rect
                    className="pc-chart-ring"
                    x={cx - 1}
                    y={TOP}
                    width={colW + 2}
                    height={plotH}
                    rx={3}
                  />
                  {showLabel && (
                    <text x={cx + colW / 2} y={H - 6} textAnchor="middle">
                      {c.label}
                    </text>
                  )}
                </g>
              );
            })}
            <line
              className="pc-chart-axis"
              x1={LEFT}
              x2={width}
              y1={TOP + plotH}
              y2={TOP + plotH}
            />
          </svg>
          {/* Legenda: il colore non è l'unico canale (le etichette delle colonne lo dicono già). */}
          <ul className="pc-chart-legend pc-chart-legend-static" aria-hidden="true">
            <li>
              <span className="pc-chart-swatch pc-chart-swatch-soft" />
              {totalLabel}
            </li>
            <li>
              <span className="pc-chart-swatch pc-chart-swatch-fill" />
              {analyzedLabel}
            </li>
          </ul>
        </>
      )}
      {node}
    </div>
  );
}
