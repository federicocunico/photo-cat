// Web Mercator in "world units" (0..1 across the whole world), plus view helpers.

/** atan(sinh(π)) in degrees: y = 0 exactly (same constant as the backend `geo_map::MAX_LAT`). */
export const MAX_LAT = 85.05112877980659;
export const MIN_ZOOM = 1;
export const MAX_ZOOM = 64;
/** Pixel size of the world at zoom 1. */
export const BASE_TILE = 256;

export interface MapView {
  centerLon: number;
  centerLat: number;
  zoom: number;
}
export interface BBox {
  south: number;
  west: number;
  north: number;
  east: number;
}
export interface Size {
  width: number;
  height: number;
}

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
export const clampLat = (lat: number) => clamp(lat, -MAX_LAT, MAX_LAT);
export const clampZoom = (z: number) => clamp(z, MIN_ZOOM, MAX_ZOOM);
export const wrapLon = (lon: number) => ((((lon + 180) % 360) + 360) % 360) - 180;

/** lon/lat -> world units: x in [0,1] (west->east), y in [0,1] (north->south). */
export function project(lon: number, lat: number): [number, number] {
  const phi = (clampLat(lat) * Math.PI) / 180;
  return [(lon + 180) / 360, 0.5 - Math.log(Math.tan(Math.PI / 4 + phi / 2)) / (2 * Math.PI)];
}

export function invert(x: number, y: number): [number, number] {
  const lat = (Math.atan(Math.sinh(Math.PI * (1 - 2 * y))) * 180) / Math.PI;
  return [x * 360 - 180, clampLat(lat)];
}

/** Size in pixels of the whole world at a zoom. */
export const worldPx = (zoom: number) => BASE_TILE * zoom;

/**
 * Horizontal world copies intersecting the viewport, as integer offsets in whole worlds
 * (multiples of 360° of longitude) from the canonical [-180, 180] copy.
 */
export function worldCopies(view: MapView, size: Size): number[] {
  const [cx] = project(view.centerLon, view.centerLat);
  const half = size.width / 2 / worldPx(view.zoom);
  const out: number[] = [];
  for (let k = Math.floor(cx - half); k < Math.ceil(cx + half); k++) out.push(k);
  return out;
}

/** Lon/lat -> canvas CSS pixels. Longitude is not wrapped. */
export function lonLatToPixel(view: MapView, size: Size, lon: number, lat: number): [number, number] {
  const [cx, cy] = project(view.centerLon, view.centerLat);
  const [x, y] = project(lon, lat);
  const s = worldPx(view.zoom);
  return [size.width / 2 + (x - cx) * s, size.height / 2 + (y - cy) * s];
}

/** Canvas CSS pixels -> lon/lat (longitude wrapped, latitude clamped). */
export function pixelToLonLat(view: MapView, size: Size, px: number, py: number): [number, number] {
  const [cx, cy] = project(view.centerLon, view.centerLat);
  const s = worldPx(view.zoom);
  const [lon, lat] = invert(cx + (px - size.width / 2) / s, clamp(cy + (py - size.height / 2) / s, 0, 1));
  return [wrapLon(lon), lat];
}

/** Visible bounding box; when the view spans the whole world west/east are -180/180. */
export function visibleBBox(view: MapView, size: Size): BBox {
  const [cx, cy] = project(view.centerLon, view.centerLat);
  const s = worldPx(view.zoom);
  const halfW = size.width / 2 / s;
  const halfH = size.height / 2 / s;
  const north = invert(0, clamp(cy - halfH, 0, 1))[1];
  const south = invert(0, clamp(cy + halfH, 0, 1))[1];
  if (halfW >= 0.5) return { south, west: -180, north, east: 180 };
  return {
    south,
    west: wrapLon((cx - halfW) * 360 - 180),
    north,
    east: wrapLon((cx + halfW) * 360 - 180),
  };
}

/** Normalise a view: zoom limits, latitude clamp, longitude wrap, no panning past the poles. */
export function normalizeView(view: MapView, size?: Size): MapView {
  const zoom = clampZoom(view.zoom);
  let centerLat = clampLat(view.centerLat);
  if (size) {
    const halfH = size.height / 2 / worldPx(zoom);
    if (halfH < 0.5) {
      const y = clamp(project(0, centerLat)[1], halfH, 1 - halfH);
      centerLat = invert(0, y)[1];
    }
  }
  return { centerLon: wrapLon(view.centerLon), centerLat, zoom };
}

/** Pan by a pixel delta (content follows the pointer: dx>0 moves the map right). */
export function panBy(view: MapView, size: Size, dx: number, dy: number): MapView {
  const [cx, cy] = project(view.centerLon, view.centerLat);
  const s = worldPx(view.zoom);
  const [lon, lat] = invert(cx - dx / s, clamp(cy - dy / s, 0, 1));
  // keep longitude continuous (no wrap before normalize) so tiny pans stay exact
  return normalizeView({ ...view, centerLon: lon, centerLat: lat }, size);
}

/** Zoom to `zoom`, keeping the geographic point under (px,py) fixed. */
export function zoomAt(view: MapView, size: Size, zoom: number, px: number, py: number): MapView {
  const z = clampZoom(zoom);
  if (z === view.zoom) return view;
  const [lon, lat] = pixelToLonLat(view, size, px, py);
  const [x, y] = project(lon, lat);
  const s = worldPx(z);
  const cx = x - (px - size.width / 2) / s;
  const cy = clamp(y - (py - size.height / 2) / s, 0, 1);
  const [clon, clat] = invert(cx, cy);
  return normalizeView({ centerLon: clon, centerLat: clat, zoom: z }, size);
}
