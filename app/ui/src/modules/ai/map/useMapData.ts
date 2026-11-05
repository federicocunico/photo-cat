// Richieste della mappa (photo-map D5): chiave arrotondata, nessun salto mentre si
// aspetta la risposta (si mostrano i gruppi precedenti).
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import type { BBox } from "../../../components/map/projection";
import { commands, isTauri, type GeoBbox, type LibraryFilter } from "../../../lib/ipc";
import { emptyFilter } from "../../../store/library";
import { gridZoom } from "./clusters";

export const MAP_KEY = ["ai", "map"] as const;

/** Arrotonda l'area verso l'esterno a multipli di `step` gradi: chiave e richiesta coincidono. */
export function snapBBox(b: BBox, step: number): BBox {
  const down = (v: number) => Math.floor(v / step) * step;
  const up = (v: number) => Math.ceil(v / step) * step;
  const r = (v: number) => Math.round(v * 1e6) / 1e6;
  return {
    south: r(Math.max(-90, down(b.south))),
    north: r(Math.min(90, up(b.north))),
    west: r(Math.max(-180, down(b.west))),
    east: r(Math.min(180, up(b.east))),
  };
}

export const toGeoBbox = (b: BBox): GeoBbox => ({ ...b });

export function useMapData(opts: { scope: number | null; approx: boolean; bbox: BBox; viewZoom: number }) {
  const { scope, approx, bbox, viewZoom } = opts;
  const zoom = gridZoom(viewZoom);
  const step = 360 / (2 ** zoom * 8);
  const snapped = useMemo(
    () => snapBBox(bbox, step),
    [bbox, step],
  );
  const filter: LibraryFilter = useMemo(() => ({ ...emptyFilter(), catalogId: scope }), [scope]);
  const allFilter: LibraryFilter = useMemo(() => emptyFilter(), []);
  const enabled = isTauri();
  const { south, west, north, east } = snapped;

  const cells = useQuery({
    queryKey: [...MAP_KEY, "cells", scope, approx, zoom, south, west, north, east],
    // Le celle portano con sé il proprio livello di griglia: con keepPreviousData, durante un
    // cambio di zoom si disegnano le celle vecchie con il loro lato, non con quello nuovo.
    queryFn: async () => ({ zoom, cells: await commands.mapCells(filter, { south, west, north, east }, zoom, approx) }),
    placeholderData: keepPreviousData,
    enabled,
  });
  const catalogs = useQuery({
    queryKey: [...MAP_KEY, "catalogs", approx, south, west, north, east],
    queryFn: () => commands.mapCatalogs(allFilter, { south, west, north, east }, approx),
    placeholderData: keepPreviousData,
    enabled,
  });
  const unplaced = useQuery({
    queryKey: [...MAP_KEY, "unplaced", scope],
    queryFn: () => commands.mapPlacesUnplaced(filter),
    enabled,
  });
  const manual = useQuery({
    queryKey: [...MAP_KEY, "manual", scope],
    queryFn: () => commands.mapPlacesManual(filter),
    enabled,
  });
  return { zoom, cells, catalogs, unplaced, manual };
}
