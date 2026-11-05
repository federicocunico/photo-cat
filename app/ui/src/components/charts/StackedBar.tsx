import { fmtInt } from "../../lib/format";
import { activateOnKey, EmptyState, useTip, useWidth } from "./shared";

export type SegmentTone = "current" | "stale" | "queued" | "never" | "extra";
export interface Segment {
  key: string;
  label: string;
  value: number;
  tone: SegmentTone;
}
export interface StackedBarProps {
  segments: Segment[];
  onSelect?: (key: string) => void;
  emptyText: string;
  ariaLabel: string;
  formatValue?: (n: number) => string;
}

const H = 22;
const GAP = 2;

export function StackedBar({
  segments,
  onSelect,
  emptyText,
  ariaLabel,
  formatValue,
}: StackedBarProps) {
  const { ref, width } = useWidth();
  const { show, hide, node } = useTip(width);
  const fmt = formatValue ?? fmtInt;
  const total = segments.reduce((s, x) => s + x.value, 0);
  const visible = segments.filter((s) => s.value > 0);
  const usable = width - GAP * Math.max(0, visible.length - 1);
  let x = 0;

  return (
    <div className="pc-chart" ref={ref}>
      {total <= 0 ? (
        <EmptyState text={emptyText} />
      ) : (
        <>
          <svg width={width} height={H} role="group" aria-label={ariaLabel}>
            {visible.map((s) => {
              const w = Math.max(2, (s.value / total) * usable);
              const sx = x;
              x += w + GAP;
              const label = `${s.label}: ${fmt(s.value)}`;
              const tipAt = () => show(sx + w / 2, 0, label);
              return (
                <g
                  key={s.key}
                  className="pc-chart-mark"
                  data-testid={`segment-${s.key}`}
                  role={onSelect ? "button" : "img"}
                  tabIndex={onSelect ? 0 : undefined}
                  aria-label={label}
                  onClick={onSelect ? () => onSelect(s.key) : undefined}
                  onKeyDown={onSelect ? activateOnKey(() => onSelect(s.key)) : undefined}
                  onPointerEnter={tipAt}
                  onPointerLeave={hide}
                  onFocus={tipAt}
                  onBlur={hide}
                >
                  <rect
                    className={`pc-chart-fill pc-chart-tone-${s.tone}`}
                    x={sx}
                    y={0}
                    width={w}
                    height={H}
                    rx={4}
                  />
                  <rect className="pc-chart-ring" x={sx} y={0} width={w} height={H} rx={4} />
                </g>
              );
            })}
          </svg>
          <ul className="pc-chart-legend">
            {segments.map((s) => (
              <li key={s.key}>
                <button
                  type="button"
                  data-testid={`legend-${s.key}`}
                  onClick={onSelect ? () => onSelect(s.key) : undefined}
                >
                  <span className={`pc-chart-swatch pc-chart-tone-${s.tone}`} aria-hidden="true" />
                  {s.label}: <strong>{fmt(s.value)}</strong>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      {node}
    </div>
  );
}
