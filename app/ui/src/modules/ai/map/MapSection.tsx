// Sezione AI > Mappa (photo-map D3, D5): gruppi di foto per area, ambito, luoghi senza coordinate.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { DEFAULT_VIEW, WorldMap, type MapProjection, type WorldMapHandle } from "../../../components/map/WorldMap";
import type { Cell } from "../../../bindings";
import {
  lonLatToPixel,
  visibleBBox,
  worldCopies,
  type BBox,
  type MapView,
  type Size,
} from "../../../components/map/projection";
import { InlineAlert } from "../../../components/ui";
import { commands, isTauri } from "../../../lib/ipc";
import { useModelStatuses } from "../../../lib/models";
import { openLibraryWith } from "../results/navigate";
import { thumbUrl } from "../../library/thumbUrl";
import {
  cellLabel,
  cellPx,
  clusterLabel,
  drawClusters,
  hitTest,
  isApproxCluster,
  layoutClusters,
  type Cluster,
} from "./clusters";
import { MapSidePanel } from "./MapSidePanel";
import { MAP_KEY, useMapData } from "./useMapData";
import "./map.css";

const FALLBACK = { width: 800, height: 600 };

export function MapSection() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const stageRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<WorldMapHandle>(null);
  const [view, setView] = useState<MapView>(DEFAULT_VIEW);
  const [bbox, setBbox] = useState<BBox>(() => visibleBBox(DEFAULT_VIEW, FALLBACK));
  const [scope, setScope] = useState<number | null>(null);
  const [approx, setApprox] = useState(true);
  const [placing, setPlacing] = useState<string | null>(null);
  const [hover, setHover] = useState<{ cluster: Cluster; x: number; y: number } | null>(null);

  // La dimensione reale si conosce solo a montaggio avvenuto.
  useEffect(() => {
    const el = stageRef.current;
    if (el)
      setBbox(
        visibleBBox(DEFAULT_VIEW, {
          width: el.clientWidth || FALLBACK.width,
          height: el.clientHeight || FALLBACK.height,
        }),
      );
  }, []);

  const { zoom, cells, catalogs, unplaced, manual } = useMapData({ scope, approx, bbox, viewZoom: view.zoom });
  const catalogList = useQuery({ queryKey: ["catalogs"], queryFn: () => commands.listCatalogs(), enabled: isTauri() });
  const models = useModelStatuses();
  const gaz = models.find((m) => m.entry.role === "gazetteer");
  const gazetteerMissing = !!gaz && gaz.state.state !== "verified";

  const cellData = useMemo(() => cells.data?.cells ?? [], [cells.data]);
  // Livello di griglia delle celle mostrate (con keepPreviousData può differire da `zoom`).
  const cellZoom = cells.data?.zoom ?? zoom;

  /** Gruppi in pixel per una vista, ripetuti su ogni copia del mondo visibile (antimeridiano). */
  const layout = useCallback(
    (v: MapView, size: Size) => {
      const side = cellPx(v.zoom, cellZoom);
      return worldCopies(v, size).flatMap((k) =>
        layoutClusters(cellData, (lon, lat) => lonLatToPixel(v, size, lon + 360 * k, lat), side),
      );
    },
    [cellData, cellZoom],
  );

  // Vista corrente, anche senza canvas (test e hit-test fuori dal disegno).
  const currentLayout = () => {
    const el = stageRef.current;
    const size = { width: el?.clientWidth || FALLBACK.width, height: el?.clientHeight || FALLBACK.height };
    return layout(mapRef.current?.getView() ?? view, size);
  };

  const renderOverlay = useCallback(
    (ctx: CanvasRenderingContext2D, p: MapProjection) => {
      const style = getComputedStyle(ctx.canvas);
      const accent = style.getPropertyValue("--pc-accent").trim() || "#4c8dff";
      drawClusters(
        ctx,
        layout(p.view, p.size),
        {
          accent,
          text: style.getPropertyValue("--pc-accent-text").trim() || "#ffffff",
          ink: style.getPropertyValue("--pc-text").trim() || undefined,
        },
        hover?.cluster ?? null,
      );
      if (placing) {
        // Mirino al centro: Invio posiziona il luogo lì (alternativa da tastiera al clic).
        const cx = p.size.width / 2;
        const cy = p.size.height / 2;
        ctx.strokeStyle = accent;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(cx - 10, cy);
        ctx.lineTo(cx + 10, cy);
        ctx.moveTo(cx, cy - 10);
        ctx.lineTo(cx, cy + 10);
        ctx.stroke();
      }
    },
    [layout, hover, placing],
  );

  const clusterAt = (x: number, y: number) => hitTest(currentLayout(), x, y);

  const openCell = (cell: Cell) =>
    // Area esatta della cella: il conteggio della Libreria coincide con quello del gruppo.
    openLibraryWith(scope, { geoBbox: { ...cell.bbox, approx, label: cellLabel(cell) } });

  const onMapClick = (lon: number, lat: number, px: { x: number; y: number }) => {
    if (placing) {
      const name = placing;
      setPlacing(null);
      void commands.mapPlaceSet(name, lat, lon).then(() => qc.invalidateQueries({ queryKey: MAP_KEY }));
      return;
    }
    const hit = clusterAt(px.x, px.y);
    if (hit) openCell(hit.cell);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.buttons) {
      // Trascinamento: niente tooltip (e nessun render per movimento).
      setHover((h) => (h ? null : h));
      return;
    }
    if (placing) return;
    const r = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    const c = clusterAt(x, y);
    setHover((h) => (c ? { cluster: c, x, y } : h ? null : h));
  };

  useEffect(() => {
    if (!placing) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setPlacing(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [placing]);

  const total = cellData.reduce((n, c) => n + c.count, 0);
  const tip = hover?.cluster;
  const tipThumbs = (tip?.cell.sampleThumbs ?? [])
    .filter((k): k is string => !!k)
    .slice(0, 4)
    .map((k) => thumbUrl(k))
    .filter((u): u is string => !!u);

  return (
    <div className="ai-map">
      <h2 id="ai-section-title">{t("ai.sections.map")}</h2>
      <p className="muted">{t("aiMap.intro")}</p>
      {cells.isError && <InlineAlert tone="danger">{t("aiMap.loadFailed")}</InlineAlert>}
      <div className="ai-map-layout">
        <div className="ai-map-main">
          <div className="ai-map-toolbar">
            <label>
              <input type="checkbox" checked={approx} onChange={(e) => setApprox(e.target.checked)} />{" "}
              {t("aiMap.showApprox")}
            </label>
            <span className="muted" aria-live="polite">
              {t("aiMap.summary", { groups: cellData.length, count: total })}
            </span>
          </div>
          <div
            ref={stageRef}
            className={`ai-map-stage${placing ? " is-placing" : ""}${tip ? " is-hit" : ""}`}
            onPointerMove={onPointerMove}
            onPointerLeave={() => setHover(null)}
          >
            <WorldMap
              mapRef={mapRef}
              initialView={DEFAULT_VIEW}
              onViewChange={(v, b) => {
                setView(v);
                setBbox(b);
              }}
              renderOverlay={renderOverlay}
              onMapClick={onMapClick}
              zoomInLabel={t("aiMap.zoomIn")}
              zoomOutLabel={t("aiMap.zoomOut")}
              label={t("aiMap.mapLabel")}
              description={t("aiMap.keyboardHint")}
            />
            {tip && hover && (
              <div className="ai-map-tip" role="tooltip" style={{ left: hover.x + 12, top: hover.y + 12 }}>
                {tipThumbs.length > 0 && (
                  <div className="ai-map-tip-thumbs">
                    {tipThumbs.map((u) => (
                      <img key={u} src={u} alt="" />
                    ))}
                  </div>
                )}
                <strong>{clusterLabel(tip) || t("aiMap.noPlace")}</strong>
                <div>{t("aiMap.tipCount", { count: tip.count })}</div>
                {tip.cell.manual && <div className="muted">{t("aiMap.tipManual")}</div>}
                {tip.approxCount > 0 && (
                  <div className="muted">
                    {isApproxCluster(tip)
                      ? t("aiMap.tipApprox", { count: tip.approxCount })
                      : t("aiMap.tipSomeApprox", { count: tip.approxCount })}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
        <MapSidePanel
          scope={scope}
          onScope={setScope}
          catalogs={catalogList.data?.map((c) => c.catalog) ?? []}
          inArea={catalogs.data ?? []}
          unplaced={unplaced.data ?? []}
          manual={manual.data ?? []}
          onClear={(name) => void commands.mapPlaceClear(name).then(() => qc.invalidateQueries({ queryKey: MAP_KEY }))}
          placing={placing}
          onPlace={setPlacing}
          gazetteerMissing={gazetteerMissing}
          cells={cellData}
          onOpenCell={openCell}
        />
      </div>
    </div>
  );
}
