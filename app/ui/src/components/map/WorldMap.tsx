import { useCallback, useEffect, useId, useImperativeHandle, useRef, useState } from "react";
import type { ReactNode, Ref } from "react";
import {
  type BBox,
  type MapView,
  type Size,
  MAX_ZOOM,
  MIN_ZOOM,
  lonLatToPixel,
  normalizeView,
  panBy,
  pixelToLonLat,
  project,
  visibleBBox,
  worldCopies,
  worldPx,
  zoomAt,
} from "./projection";
import { decodeObject, type Topology } from "./topo";

/** Rings pre-projected to world units (x,y interleaved), one array per ring. */
type WorldPolygons = Float64Array[][];

let geometryPromise: Promise<WorldPolygons> | null = null;

/** Loads and decodes the bundled country geometry once per session. */
export function loadWorldGeometry(): Promise<WorldPolygons> {
  if (!geometryPromise) {
    const base = import.meta.env?.BASE_URL ?? "/";
    const url = `${base.endsWith("/") ? base : `${base}/`}geo/countries-50m.json`;
    geometryPromise = fetch(url)
      .then((r) => {
        if (!r.ok) throw new Error(`geo asset: HTTP ${r.status}`);
        return r.json() as Promise<Topology>;
      })
      .then((topo) => {
        const out: WorldPolygons = [];
        for (const c of decodeObject(topo, "countries")) {
          for (const poly of c.polygons) {
            out.push(
              poly.map((ring) => {
                const a = new Float64Array(ring.length * 2);
                ring.forEach(([lon, lat], i) => {
                  const [x, y] = project(lon, lat);
                  a[i * 2] = x;
                  a[i * 2 + 1] = y;
                });
                return a;
              }),
            );
          }
        }
        return out;
      })
      .catch((e) => {
        geometryPromise = null; // allow a retry on next mount
        throw e;
      });
  }
  return geometryPromise;
}

export interface MapProjection {
  /** lon/lat -> canvas CSS pixels (for the current view). */
  project: (lon: number, lat: number) => [number, number];
  /** canvas CSS pixels -> lon/lat. */
  unproject: (x: number, y: number) => [number, number];
  view: MapView;
  size: Size;
}

export interface WorldMapHandle {
  getView: () => MapView;
  setView: (view: MapView) => void;
  /** Redraw the overlay layer (call when overlay data changed). */
  redrawOverlay: () => void;
}

export interface WorldMapProps {
  initialView?: MapView;
  /** Fired at the end of an interaction (pointer up, wheel/keyboard idle 150 ms, buttons). */
  onViewChange?: (view: MapView, bbox: BBox) => void;
  /** Draws on a transparent canvas above the map. Coordinates are CSS pixels. */
  renderOverlay?: (ctx: CanvasRenderingContext2D, p: MapProjection) => void;
  /** Click (not a drag) on the map. */
  onMapClick?: (lon: number, lat: number, px: { x: number; y: number }) => void;
  zoomInLabel: string;
  zoomOutLabel: string;
  /** Accessible name of the map region. */
  label: string;
  /** Accessible description (e.g. keyboard instructions). */
  description?: string;
  className?: string;
  /** Extra content rendered above the canvases (e.g. a legend). */
  children?: ReactNode;
  mapRef?: Ref<WorldMapHandle>;
}

export const DEFAULT_VIEW: MapView = { centerLon: 0, centerLat: 20, zoom: 1 };
const FALLBACK_SIZE: Size = { width: 800, height: 600 };
const WHEEL_IDLE_MS = 150;
const CLICK_SLOP = 4;
const KEY_PAN_PX = 80;

function cssVar(style: CSSStyleDeclaration, name: string, fallback: string): string {
  return style.getPropertyValue(name).trim() || fallback;
}

export function WorldMap({
  initialView = DEFAULT_VIEW,
  onViewChange,
  renderOverlay,
  onMapClick,
  zoomInLabel,
  zoomOutLabel,
  label,
  description,
  className,
  children,
  mapRef,
}: WorldMapProps) {
  const descId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const baseRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  // Only the zoom lives in React state (button limits, data-zoom): panning never re-renders.
  const [initial] = useState<MapView>(() => normalizeView(initialView));
  const [zoom, setZoom] = useState(initial.zoom);
  const [size, setSize] = useState<Size>(FALLBACK_SIZE);
  const [geometry, setGeometry] = useState<WorldPolygons | null>(null);

  const viewRef = useRef(initial);
  const sizeRef = useRef(size);
  const cbRef = useRef({ onViewChange, onMapClick, renderOverlay });
  useEffect(() => {
    cbRef.current = { onViewChange, onMapClick, renderOverlay };
  });
  const geometryRef = useRef(geometry);
  const rafRef = useRef(0);
  const idleRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const dragRef = useRef<{ id: number; x: number; y: number; sx: number; sy: number; moved: boolean } | null>(null);

  const draw = useCallback(() => {
    rafRef.current = 0;
    const v = viewRef.current;
    const s = sizeRef.current;
    const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
    const base = baseRef.current;
    const overlay = overlayRef.current;
    const root = rootRef.current;
    const bctx = base?.getContext?.("2d") ?? null;
    if (base && bctx && root) {
      const style = getComputedStyle(root);
      bctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      bctx.clearRect(0, 0, s.width, s.height);
      bctx.fillStyle = cssVar(style, "--pc-bg", "#0f1115");
      bctx.fillRect(0, 0, s.width, s.height);
      const polys = geometryRef.current;
      if (polys) {
        bctx.fillStyle = cssVar(style, "--pc-panel", "#1b1f27");
        bctx.strokeStyle = cssVar(style, "--pc-border", "#3a4150");
        bctx.lineWidth = 0.75;
        bctx.lineJoin = "round";
        const [cx, cy] = project(v.centerLon, v.centerLat);
        const px = worldPx(v.zoom);
        const ox = s.width / 2 - cx * px;
        const oy = s.height / 2 - cy * px;
        for (const wrap of worldCopies(v, s)) {
          bctx.beginPath();
          for (const poly of polys) {
            for (const ring of poly) {
              for (let i = 0; i < ring.length; i += 2) {
                const x = ox + (ring[i] + wrap) * px;
                const y = oy + ring[i + 1] * px;
                if (i === 0) bctx.moveTo(x, y);
                else bctx.lineTo(x, y);
              }
              bctx.closePath();
            }
          }
          bctx.fill("evenodd");
          bctx.stroke();
        }
      }
    }
    const octx = overlay?.getContext?.("2d") ?? null;
    if (overlay && octx) {
      octx.setTransform(dpr, 0, 0, dpr, 0, 0);
      octx.clearRect(0, 0, s.width, s.height);
      cbRef.current.renderOverlay?.(octx, {
        project: (lon, lat) => lonLatToPixel(v, s, lon, lat),
        unproject: (x, y) => pixelToLonLat(v, s, x, y),
        view: v,
        size: s,
      });
    }
  }, []);

  const schedule = useCallback(() => {
    if (rafRef.current) return;
    if (typeof requestAnimationFrame === "function") rafRef.current = requestAnimationFrame(draw);
    else draw();
  }, [draw]);

  const emit = useCallback(() => {
    clearTimeout(idleRef.current);
    cbRef.current.onViewChange?.(viewRef.current, visibleBBox(viewRef.current, sizeRef.current));
  }, []);
  const emitIdle = useCallback(() => {
    clearTimeout(idleRef.current);
    idleRef.current = setTimeout(emit, WHEEL_IDLE_MS);
  }, [emit]);

  const apply = useCallback(
    (next: MapView) => {
      if (
        next.centerLon === viewRef.current.centerLon &&
        next.centerLat === viewRef.current.centerLat &&
        next.zoom === viewRef.current.zoom
      ) {
        return false;
      }
      viewRef.current = next;
      setZoom(next.zoom);
      schedule();
      return true;
    },
    [schedule],
  );

  useImperativeHandle(
    mapRef,
    () => ({
      getView: () => viewRef.current,
      setView: (v) => {
        apply(normalizeView(v, sizeRef.current));
      },
      redrawOverlay: schedule,
    }),
    [apply, schedule],
  );

  // geometry
  useEffect(() => {
    let alive = true;
    loadWorldGeometry()
      .then((g) => {
        if (!alive) return;
        geometryRef.current = g;
        setGeometry(g);
        schedule();
      })
      .catch(() => {
        /* the map still works without the basemap (clusters stay usable) */
      });
    return () => {
      alive = false;
    };
  }, [schedule]);

  // size + canvas backing store
  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const measure = () => {
      const w = root.clientWidth || FALLBACK_SIZE.width;
      const h = root.clientHeight || FALLBACK_SIZE.height;
      const dpr = window.devicePixelRatio || 1;
      for (const c of [baseRef.current, overlayRef.current]) {
        if (!c) continue;
        c.width = Math.round(w * dpr);
        c.height = Math.round(h * dpr);
      }
      if (w !== sizeRef.current.width || h !== sizeRef.current.height) {
        sizeRef.current = { width: w, height: h };
        setSize(sizeRef.current);
      }
      schedule();
    };
    measure();
    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", measure);
      return () => window.removeEventListener("resize", measure);
    }
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    return () => ro.disconnect();
  }, [schedule]);

  useEffect(
    () => () => {
      clearTimeout(idleRef.current);
      if (rafRef.current && typeof cancelAnimationFrame === "function") cancelAnimationFrame(rafRef.current);
    },
    [],
  );

  const localPoint = (clientX: number, clientY: number) => {
    const r = rootRef.current?.getBoundingClientRect();
    return { x: clientX - (r?.left ?? 0), y: clientY - (r?.top ?? 0) };
  };

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    dragRef.current = { id: e.pointerId, x: e.clientX, y: e.clientY, sx: e.clientX, sy: e.clientY, moved: false };
    try {
      e.currentTarget.setPointerCapture?.(e.pointerId);
    } catch {
      /* ignore */
    }
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = dragRef.current;
    if (!d || d.id !== e.pointerId) return;
    if (!d.moved && Math.hypot(e.clientX - d.sx, e.clientY - d.sy) < CLICK_SLOP) return;
    d.moved = true;
    apply(panBy(viewRef.current, sizeRef.current, e.clientX - d.x, e.clientY - d.y));
    d.x = e.clientX;
    d.y = e.clientY;
  };
  const endDrag = (e: React.PointerEvent<HTMLDivElement>, cancelled: boolean) => {
    const d = dragRef.current;
    if (!d || d.id !== e.pointerId) return;
    dragRef.current = null;
    try {
      e.currentTarget.releasePointerCapture?.(e.pointerId);
    } catch {
      /* ignore */
    }
    if (d.moved) {
      emit();
    } else if (!cancelled) {
      const p = localPoint(e.clientX, e.clientY);
      const [lon, lat] = pixelToLonLat(viewRef.current, sizeRef.current, p.x, p.y);
      cbRef.current.onMapClick?.(lon, lat, p);
    }
  };

  // React's onWheel is passive (preventDefault is ignored and the page would scroll):
  // a native non-passive listener is required.
  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = root.getBoundingClientRect();
      const factor = Math.exp(-e.deltaY * 0.0015);
      const v = viewRef.current;
      if (apply(zoomAt(v, sizeRef.current, v.zoom * factor, e.clientX - r.left, e.clientY - r.top))) emitIdle();
    };
    root.addEventListener("wheel", onWheel, { passive: false });
    return () => root.removeEventListener("wheel", onWheel);
  }, [apply, emitIdle]);

  const zoomCentered = (factor: number) => {
    const s = sizeRef.current;
    if (apply(zoomAt(viewRef.current, s, viewRef.current.zoom * factor, s.width / 2, s.height / 2))) emit();
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const s = sizeRef.current;
    let next: MapView | null = null;
    switch (e.key) {
      case "ArrowLeft":
        next = panBy(viewRef.current, s, KEY_PAN_PX, 0);
        break;
      case "ArrowRight":
        next = panBy(viewRef.current, s, -KEY_PAN_PX, 0);
        break;
      case "ArrowUp":
        next = panBy(viewRef.current, s, 0, KEY_PAN_PX);
        break;
      case "ArrowDown":
        next = panBy(viewRef.current, s, 0, -KEY_PAN_PX);
        break;
      case "+":
      case "=":
        next = zoomAt(viewRef.current, s, viewRef.current.zoom * 2, s.width / 2, s.height / 2);
        break;
      case "-":
      case "_":
        next = zoomAt(viewRef.current, s, viewRef.current.zoom / 2, s.width / 2, s.height / 2);
        break;
      case "Enter":
      case " ": {
        // Keyboard equivalent of a click, at the centre of the map (not for the zoom buttons).
        if (e.target !== e.currentTarget) return;
        e.preventDefault();
        const p = { x: s.width / 2, y: s.height / 2 };
        const [lon, lat] = pixelToLonLat(viewRef.current, s, p.x, p.y);
        cbRef.current.onMapClick?.(lon, lat, p);
        return;
      }
      default:
        return;
    }
    e.preventDefault();
    if (apply(next)) emitIdle();
  };

  // redraw when React state changed (size/geometry) or overlay callback identity changed; view
  // changes redraw directly through apply()
  useEffect(() => {
    schedule();
  }, [size, geometry, renderOverlay, schedule]);

  return (
    <div
      ref={rootRef}
      className={className}
      role="application"
      aria-label={label}
      aria-describedby={description ? descId : undefined}
      tabIndex={0}
      data-testid="world-map"
      data-zoom={zoom}
      style={{ position: "relative", overflow: "hidden", touchAction: "none", cursor: "grab", minHeight: 200 }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={(e) => endDrag(e, false)}
      onPointerCancel={(e) => endDrag(e, true)}
      onKeyDown={onKeyDown}
    >
      {description && (
        <span id={descId} className="visually-hidden">
          {description}
        </span>
      )}
      <canvas ref={baseRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
      <canvas
        ref={overlayRef}
        data-testid="world-map-overlay"
        style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}
      />
      <div style={{ position: "absolute", top: 8, right: 8, display: "flex", flexDirection: "column", gap: 4 }}>
        <button type="button" aria-label={zoomInLabel} disabled={zoom >= MAX_ZOOM} onClick={() => zoomCentered(2)}>
          +
        </button>
        <button type="button" aria-label={zoomOutLabel} disabled={zoom <= MIN_ZOOM} onClick={() => zoomCentered(0.5)}>
          −
        </button>
      </div>
      {children}
    </div>
  );
}

