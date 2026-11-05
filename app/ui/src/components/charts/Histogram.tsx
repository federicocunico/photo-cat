import { fmtInt } from "../../lib/format";
import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { EmptyState, useTip, useWidth } from "./shared";

export interface HistogramBin {
  from: number;
  to: number;
  count: number;
}
export type HistogramRange = [number, number];
export interface HistogramProps {
  bins: HistogramBin[];
  threshold?: number;
  thresholdLabel?: string;
  /** Intervallo selezionato (controllato); se omesso lo stato è interno. */
  range?: HistogramRange | null;
  onRangeChange?: (range: HistogramRange | null) => void;
  emptyText: string;
  ariaLabel: string;
  /** Istruzioni per tastiera, lette dalle tecnologie assistive. */
  hint?: string;
  /** Testo del tooltip di un intervallo. */
  formatBin?: (bin: HistogramBin) => string;
  /** Annuncio (aria-live) dell'intervallo selezionato. */
  formatRange?: (range: HistogramRange) => string;
}

const H = 170;
const TOP = 10;
const BOTTOM = 24;
const LEFT = 36;

const f2 = (n: number) => n.toFixed(2);

/**
 * Selezione con puntatore (trascinamento) o tastiera: Frecce spostano il cursore,
 * Maiusc+Frecce estendono, Invio/Spazio iniziano (e poi chiudono) una selezione, Esc la toglie.
 */
export function Histogram({
  bins,
  threshold,
  thresholdLabel,
  range: rangeProp,
  onRangeChange,
  emptyText,
  ariaLabel,
  hint,
  formatBin,
  formatRange,
}: HistogramProps) {
  const { ref, width } = useWidth();
  const { show, hide, node } = useTip(width);
  const svgRef = useRef<SVGSVGElement>(null);
  const [inner, setInner] = useState<HistogramRange | null>(null);
  const range = rangeProp !== undefined ? rangeProp : inner;
  const [drag, setDrag] = useState<[number, number] | null>(null);
  const [cursor, setCursor] = useState(0);
  const [anchor, setAnchor] = useState<number | null>(null);

  const n = bins.length;
  const empty = n === 0 || bins.every((b) => b.count === 0);
  const plotW = Math.max(40, width - LEFT);
  const plotH = H - TOP - BOTTOM;
  const bw = plotW / Math.max(1, n);
  const max = Math.max(1, ...bins.map((b) => b.count));
  const lo = n ? bins[0].from : 0;
  const hi = n ? bins[n - 1].to : 1;
  const xOf = (v: number) => LEFT + ((v - lo) / (hi - lo || 1)) * plotW;
  const binBy = (a: number, b: number): HistogramRange => [
    bins[Math.min(a, b)].from,
    bins[Math.max(a, b)].to,
  ];
  const apply = (r: HistogramRange | null) => {
    if (rangeProp === undefined) setInner(r);
    onRangeChange?.(r);
  };
  const binAt = (clientX: number) => {
    const rect = svgRef.current?.getBoundingClientRect();
    const x = (Number.isFinite(clientX) ? clientX : 0) - (rect?.left ?? 0) - LEFT;
    return Math.min(n - 1, Math.max(0, Math.floor(x / bw)));
  };

  // Tooltip e cursore anche da tastiera: al focus e a ogni spostamento con le frecce.
  const [kbd, setKbd] = useState(false);
  const pointer = useRef(false);
  const showBin = (i: number) => {
    const b = bins[i];
    if (b) show(LEFT + i * bw + bw / 2, TOP + plotH - (b.count / max) * plotH, binText(b));
  };
  const onFocus = () => {
    if (pointer.current || empty) return;
    setKbd(true);
    showBin(cursor);
  };
  const onBlur = () => {
    pointer.current = false;
    setKbd(false);
    hide();
  };

  const onDown = (e: PointerEvent) => {
    pointer.current = true;
    setKbd(false);
    if (empty) return;
    const i = binAt(e.clientX);
    setDrag([i, i]);
    setAnchor(null);
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  };
  const onMove = (e: PointerEvent) => {
    if (drag) setDrag([drag[0], binAt(e.clientX)]);
  };
  const onUp = (e: PointerEvent) => {
    if (!drag) return;
    const end = binAt(e.clientX);
    const start = drag[0];
    setDrag(null);
    setCursor(end);
    apply(binBy(start, end));
  };

  let sel: [number, number] | null = null;
  if (drag) sel = [Math.min(...drag), Math.max(...drag)];
  else if (range) {
    const a = bins.findIndex((b) => b.to > range[0] + 1e-9);
    let z = -1;
    bins.forEach((b, i) => {
      if (b.from < range[1] - 1e-9) z = i;
    });
    if (a >= 0 && z >= a) sel = [a, z];
  }

  const onKey = (e: KeyboardEvent) => {
    if (empty) return;
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      const next = Math.min(n - 1, Math.max(0, cursor + (e.key === "ArrowRight" ? 1 : -1)));
      if (e.shiftKey) {
        const a = anchor ?? cursor;
        setAnchor(a);
        apply(binBy(a, next));
      } else if (anchor !== null) {
        apply(binBy(anchor, next));
      }
      setCursor(next);
      setKbd(true);
      showBin(next);
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (anchor === null) {
        setAnchor(cursor);
        apply(binBy(cursor, cursor));
      } else {
        apply(binBy(anchor, cursor));
        setAnchor(null);
      }
    } else if (e.key === "Escape") {
      setAnchor(null);
      apply(null);
    }
  };

  const rangeText = range
    ? formatRange
      ? formatRange(range)
      : `${f2(range[0])}–${f2(range[1])}`
    : "";
  const binText = (b: HistogramBin) =>
    formatBin ? formatBin(b) : `${f2(b.from)}–${f2(b.to)}: ${b.count}`;
  const thrX = threshold !== undefined ? xOf(threshold) : 0;

  return (
    <div className="pc-chart" ref={ref}>
      {empty ? (
        <EmptyState text={emptyText} />
      ) : (
        <>
          <svg
            ref={svgRef}
            className="pc-chart-histogram"
            width={width}
            height={H}
            role="group"
            tabIndex={0}
            aria-label={ariaLabel}
            aria-description={hint}
            data-testid="histogram"
            style={{ touchAction: "none" }}
            onPointerDown={onDown}
            onPointerMove={onMove}
            onPointerUp={onUp}
            onKeyDown={onKey}
            onFocus={onFocus}
            onBlur={onBlur}
          >
            <line className="pc-chart-grid" x1={LEFT} x2={width} y1={TOP} y2={TOP} />
            <text x={LEFT - 6} y={TOP + 4} textAnchor="end">
              {fmtInt(max)}
            </text>
            {sel && (
              <rect
                className="pc-chart-sel"
                data-testid="histogram-selection"
                x={LEFT + sel[0] * bw}
                y={TOP}
                width={(sel[1] - sel[0] + 1) * bw}
                height={plotH}
              />
            )}
            {bins.map((b, i) => {
              const h = (b.count / max) * plotH;
              const inSel = sel !== null && i >= sel[0] && i <= sel[1];
              return (
                <g
                  key={i}
                  data-testid={`bin-${i}`}
                  aria-label={binText(b)}
                  onPointerEnter={() => show(LEFT + i * bw + bw / 2, TOP + plotH - h, binText(b))}
                  onPointerLeave={hide}
                >
                  <rect
                    className="pc-chart-hit"
                    x={LEFT + i * bw}
                    y={TOP}
                    width={bw}
                    height={plotH}
                  />
                  <rect
                    className={inSel ? "pc-chart-fill-selected" : "pc-chart-fill"}
                    x={LEFT + i * bw + 1}
                    y={TOP + plotH - h}
                    width={Math.max(1, bw - 2)}
                    height={h}
                    rx={2}
                  />
                </g>
              );
            })}
            {(kbd || anchor !== null) && (
              <rect
                className="pc-chart-ring"
                style={{ stroke: "var(--pc-focus)", strokeWidth: 2, pointerEvents: "none" }}
                x={LEFT + cursor * bw}
                y={TOP}
                width={bw}
                height={plotH}
              />
            )}
            <line
              className="pc-chart-axis"
              x1={LEFT}
              x2={width}
              y1={TOP + plotH}
              y2={TOP + plotH}
            />
            {[0, 0.25, 0.5, 0.75, 1].map((f) => (
              <text
                key={f}
                x={LEFT + f * plotW}
                y={H - 6}
                textAnchor={f === 0 ? "start" : f === 1 ? "end" : "middle"}
              >
                {f2(lo + f * (hi - lo))}
              </text>
            ))}
            {threshold !== undefined && threshold >= lo && threshold <= hi && (
              <g data-testid="histogram-threshold" pointerEvents="none">
                <line
                  className="pc-chart-threshold"
                  x1={thrX}
                  x2={thrX}
                  y1={TOP}
                  y2={TOP + plotH}
                />
                {thresholdLabel && (
                  <text
                    className="pc-chart-value"
                    x={thrX + (thrX > width - 90 ? -4 : 4)}
                    y={TOP + 12}
                    textAnchor={thrX > width - 90 ? "end" : "start"}
                  >
                    {thresholdLabel}
                  </text>
                )}
              </g>
            )}
          </svg>
          <div className="pc-chart-sr" aria-live="polite" data-testid="histogram-live">
            {kbd && bins[cursor]
              ? [binText(bins[cursor]), rangeText].filter(Boolean).join(". ")
              : rangeText}
          </div>
        </>
      )}
      {node}
    </div>
  );
}
