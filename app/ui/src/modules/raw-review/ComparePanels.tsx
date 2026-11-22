// Confronto delle varianti di uno scatto (porting del confronto di FujiView `App.tsx`): un
// pannello per preset visibile in una griglia adattiva, con zoom e spostamento sincronizzati da
// un'unica trasformazione. Le anteprime vengono da `review://`, alla dimensione che serve al pannello.
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { IconButton, Tooltip } from "../../components/ui";
import { commands } from "../../lib/ipc";
import type { ImageMetadata, ReviewShotView } from "../../lib/ipc";
import { canChoose } from "./choose";
import { metaLabel, metaValue, recipeSummary } from "./metaText";
import { panelGrid } from "./project";
import { presetLabel } from "./presetLabel";
import { maxEdgeFor, previewEdge } from "./previewEdge";
import { FIT, panBy, toggleZoom, viewportTransform, zoomBy } from "./viewport";
import type { Viewport } from "./viewport";

/** Attesa prima di precaricare gli scatti vicini: scorrendo in fretta si accoda solo l'ultimo. */
const PREFETCH_DEBOUNCE_MS = 60;
/** Spostamento minimo (px) oltre il quale un trascinamento non è più un clic. */
const DRAG_THRESHOLD = 3;
/** Dimensione dell'area dei pannelli finché non è misurata (e nei test). */
const DEFAULT_AREA = { width: 1280, height: 800 };

type Props = {
  sessionId: string;
  shot: ReviewShotView;
  /** Preset dei pannelli visibili, nell'ordine di visualizzazione. */
  panels: readonly string[];
  winnerPreset: string | null;
  metadata: Record<string, ImageMetadata>;
  /** Posizione del pannello a fuoco per la tastiera (Invio sceglie, le frecce lo spostano). */
  focusedIndex?: number;
  onFocusPanel?: (index: number) => void;
  onChoose: (preset: string) => void;
  onHide: (preset: string) => void;
};

export function ComparePanels({
  sessionId,
  shot,
  panels,
  winnerPreset,
  metadata,
  focusedIndex = -1,
  onFocusPanel,
  onChoose,
  onHide,
}: Props) {
  const { t } = useTranslation();
  const grid = panelGrid(panels.length);
  const [viewport, setViewport] = useState<Viewport>(FIT);
  // Un altro scatto riparte dalla vista adattata.
  const [viewShot, setViewShot] = useState(shot.key);
  if (viewShot !== shot.key) {
    setViewShot(shot.key);
    setViewport(FIT);
  }

  const area = useAreaSize();
  const cap = maxEdgeFor(panels.length);
  const needed = previewEdge(
    area.size.width / grid.columns,
    area.size.height / grid.rows,
    typeof window === "undefined" ? 1 : window.devicePixelRatio || 1,
    viewport.scale,
    cap,
  );
  // Il lato non scende finché lo scatto resta lo stesso: tornando alla vista adattata dopo uno
  // zoom si tengono le anteprime più grandi, senza richiedere e decodificare tutto di nuovo.
  const [edgeState, setEdgeState] = useState({ shot: shot.key, edge: needed });
  if (edgeState.shot !== shot.key || needed > edgeState.edge) {
    setEdgeState({ shot: shot.key, edge: needed });
  }
  const edge = Math.min(
    cap,
    edgeState.shot === shot.key ? Math.max(edgeState.edge, needed) : needed,
  );
  const urls = useImageUrls(sessionId, shot.key, edge);
  usePrefetch(sessionId, shot.key, panels);

  const choose = (preset: string) => {
    if (canChoose(shot, preset, t)) onChoose(preset);
  };

  return (
    <div
      ref={area.ref}
      className="rr-compare"
      role="group"
      aria-label={t("rawReviewCompare.region")}
      style={{
        gridTemplateColumns: `repeat(${grid.columns}, minmax(0, 1fr))`,
        gridTemplateRows: `repeat(${grid.rows}, minmax(0, 1fr))`,
      }}
    >
      {panels.map((preset, position) => (
        <Panel
          key={preset}
          preset={preset}
          position={position}
          variant={shot.variants.find((v) => v.preset === preset)}
          src={urls[preset]}
          viewport={viewport}
          onViewport={setViewport}
          winner={winnerPreset === preset}
          focused={position === focusedIndex}
          metadata={metadata[preset]}
          canHide={panels.length > 1}
          onFocusIn={() => {
            if (position !== focusedIndex) onFocusPanel?.(position);
          }}
          onChoose={() => {
            onFocusPanel?.(position);
            choose(preset);
          }}
          onHide={() => onHide(preset)}
        />
      ))}
    </div>
  );
}

/** Misura l'area dei pannelli (per scegliere il lato delle anteprime). */
function useAreaSize() {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState(DEFAULT_AREA);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) setSize({ width, height });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  return { ref, size };
}

/** URL `review://` di tutte le varianti dello scatto al lato richiesto. */
function useImageUrls(sessionId: string, shotKey: string, edge: number): Record<string, string> {
  const [state, setState] = useState<{ id: string; urls: Record<string, string> }>({
    id: "",
    urls: {},
  });
  useEffect(() => {
    let current = true;
    const id = `${sessionId}|${shotKey}`;
    commands
      .rawReviewImageKeys(sessionId, shotKey, edge)
      .then((keys) => {
        if (!current) return;
        const urls: Record<string, string> = {};
        for (const { preset, key } of keys) urls[preset] = convertFileSrc(key, "review");
        setState({ id, urls });
      })
      .catch(() => {
        if (current) setState({ id, urls: {} });
      });
    return () => {
      current = false;
    };
  }, [sessionId, shotKey, edge]);
  // Le anteprime di un altro scatto non si mostrano mai: si aspetta la risposta.
  return state.id === `${sessionId}|${shotKey}` ? state.urls : NO_URLS;
}
const NO_URLS: Record<string, string> = {};

/** Precarica gli scatti vicini per i preset visibili, con un'attesa breve (debounce). */
function usePrefetch(sessionId: string, shotKey: string, panels: readonly string[]) {
  const presets = panels.join("\n");
  useEffect(() => {
    const list = presets.split("\n").filter(Boolean);
    if (list.length === 0) return;
    const timer = window.setTimeout(() => {
      void commands.rawReviewPrefetch(sessionId, shotKey, list).catch(() => undefined);
    }, PREFETCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [sessionId, shotKey, presets]);
}

type PanelProps = {
  preset: string;
  position: number;
  variant: ReviewShotView["variants"][number] | undefined;
  src: string | undefined;
  viewport: Viewport;
  onViewport: (update: (view: Viewport) => Viewport) => void;
  winner: boolean;
  focused: boolean;
  metadata: ImageMetadata | undefined;
  canHide: boolean;
  /** Il fuoco della tastiera entra nel pannello (Tab sull'intestazione o sui pulsanti). */
  onFocusIn: () => void;
  onChoose: () => void;
  onHide: () => void;
};

function Panel({
  preset,
  position,
  variant,
  src,
  viewport,
  onViewport,
  winner,
  focused,
  metadata,
  canHide,
  onFocusIn,
  onChoose,
  onHide,
}: PanelProps) {
  const { t } = useTranslation();
  const name = presetLabel(t, preset);
  const recipe = metadata?.recipe ?? null;
  const frame = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number } | null>(null);
  const dragged = useRef(false);
  const [failed, setFailed] = useState(false);
  const [failedSrc, setFailedSrc] = useState(src);
  if (failedSrc !== src) {
    setFailedSrc(src);
    setFailed(false);
  }

  // La rotellina deve poter fermare lo scorrimento della pagina: l'ascoltatore non è passivo.
  useEffect(() => {
    const el = frame.current;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      onViewport((view) => zoomBy(view, event.deltaY));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [onViewport]);

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    dragged.current = false;
    if (viewport.scale <= 1) return;
    event.currentTarget.setPointerCapture?.(event.pointerId);
    drag.current = { x: event.clientX, y: event.clientY };
  };
  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const start = drag.current;
    if (!start) return;
    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) dragged.current = true;
    drag.current = { x: event.clientX, y: event.clientY };
    onViewport((view) => panBy(view, dx, dy));
  };
  const endDrag = () => {
    drag.current = null;
  };

  const rows = recipe?.rows ?? [];
  const title = (
    <span className="rr-panel-title" tabIndex={0}>
      <span className="rr-panel-name">{name}</span>
      {recipe && <small className="rr-panel-recipe">{recipeSummary(t, recipe)}</small>}
    </span>
  );

  return (
    <section
      className={`rr-panel${winner ? " is-winner" : ""}${focused ? " is-focused" : ""}${
        variant ? "" : " is-missing"
      }`}
      data-focused={focused ? "true" : undefined}
      aria-label={name}
      onFocus={onFocusIn}
    >
      <header className="rr-panel-head">
        <span
          className="rr-panel-number"
          title={t("rawReviewCompare.panelNumber", { n: position + 1 })}
        >
          {position + 1}
        </span>
        {rows.length > 0 ? (
          <Tooltip
            side="bottom"
            content={
              <dl className="rr-recipe-tip">
                {rows.map((row) => (
                  <div key={row.label}>
                    <dt>{metaLabel(t, row)}</dt>
                    <dd>{metaValue(t, row)}</dd>
                  </div>
                ))}
              </dl>
            }
          >
            {title}
          </Tooltip>
        ) : (
          title
        )}
        <span className="rr-panel-actions">
          {variant && (
            <IconButton
              size="sm"
              label={t("rawReviewCompare.choose", { name })}
              aria-pressed={winner}
              icon={<Icon.Trophy size={16} />}
              className={winner ? "is-on" : ""}
              disabledReason={
                variant.selectable ? undefined : t("rawReviewCompare.rawNotChoosable")
              }
              onClick={onChoose}
            />
          )}
          <IconButton
            size="sm"
            label={t("rawReviewCompare.hide", { name })}
            icon={<Icon.Close size={16} />}
            className="rr-panel-hide"
            disabledReason={canHide ? undefined : t("rawReviewCompare.lastPanel")}
            onClick={onHide}
          />
        </span>
      </header>
      {/* Clic, rotellina, trascinamento e doppio clic sono scorciatoie del mouse: da tastiera
          si sceglie con il pulsante dell'intestazione. */}
      <div
        ref={frame}
        className="rr-viewport"
        data-zoomed={viewport.scale > 1 ? "true" : undefined}
        onClick={(event) => {
          // Un doppio clic adatta o ingrandisce: non è due scelte.
          if (!dragged.current && event.detail < 2) onChoose();
          dragged.current = false;
        }}
        onDoubleClick={() => onViewport(toggleZoom)}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
      >
        {variant && src && !failed && (
          <img
            className="rr-image"
            draggable={false}
            decoding="async"
            src={src}
            alt={variant.fileName}
            style={{ transform: viewportTransform(viewport) }}
            onError={() => setFailed(true)}
          />
        )}
        {!variant && (
          <div className="rr-empty">
            <Icon.Close size={24} />
            <span>{t("rawReviewCompare.unavailable")}</span>
          </div>
        )}
        {variant && failed && <div className="rr-error">{t("rawReviewCompare.loadFailed")}</div>}
        {winner && (
          <span className="rr-winner-badge">
            <Icon.Trophy size={14} />
            {t("rawReviewCompare.best")}
          </span>
        )}
      </div>
    </section>
  );
}
