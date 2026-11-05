import { describe, expect, it } from "vitest";
import {
  MAX_LAT, MAX_ZOOM, MIN_ZOOM, invert, lonLatToPixel, panBy, pixelToLonLat, project, visibleBBox, worldCopies, zoomAt,
} from "./projection";

const size = { width: 800, height: 600 };
const view = { centerLon: 10, centerLat: 45, zoom: 4 };

describe("projection", () => {
  it("round-trips lon/lat", () => {
    for (const [lon, lat] of [[0, 0], [12.5, 41.9], [-170, -60], [179, 80]]) {
      const [x, y] = project(lon, lat);
      const [l2, a2] = invert(x, y);
      expect(l2).toBeCloseTo(lon, 6);
      expect(a2).toBeCloseTo(lat, 6);
    }
  });
  it("maps the origin to the centre and clamps latitude", () => {
    expect(project(0, 0)[0]).toBeCloseTo(0.5);
    expect(project(0, 0)[1]).toBeCloseTo(0.5);
    expect(project(0, 90)[1]).toBeCloseTo(0, 4);
    expect(project(0, -90)[1]).toBeCloseTo(1, 4);
    expect(invert(0.5, -1)[1]).toBeCloseTo(MAX_LAT, 3);
  });
  it("pixel conversion is symmetric and centred", () => {
    const [cx, cy] = lonLatToPixel(view, size, view.centerLon, view.centerLat);
    expect(cx).toBeCloseTo(400);
    expect(cy).toBeCloseTo(300);
    const [x, y] = lonLatToPixel(view, size, 14, 47);
    const [lon, lat] = pixelToLonLat(view, size, x, y);
    expect(lon).toBeCloseTo(14, 6);
    expect(lat).toBeCloseTo(47, 6);
  });
  it("limits zoom", () => {
    expect(zoomAt(view, size, 1000, 400, 300).zoom).toBe(MAX_ZOOM);
    expect(zoomAt(view, size, 0.01, 400, 300).zoom).toBe(MIN_ZOOM);
  });
  it("zooms around the cursor", () => {
    const before = pixelToLonLat(view, size, 600, 200);
    const z = zoomAt(view, size, 8, 600, 200);
    const after = pixelToLonLat(z, size, 600, 200);
    expect(after[0]).toBeCloseTo(before[0], 6);
    expect(after[1]).toBeCloseTo(before[1], 6);
  });
  it("computes the visible bbox", () => {
    const b = visibleBBox(view, size);
    expect(b.north).toBeGreaterThan(45);
    expect(b.south).toBeLessThan(45);
    expect(b.west).toBeLessThan(10);
    expect(b.east).toBeGreaterThan(10);
    const w = visibleBBox({ centerLon: 0, centerLat: 0, zoom: 1 }, { width: 2000, height: 256 });
    expect(w.west).toBe(-180);
    expect(w.east).toBe(180);
  });
  it("pans with the pointer and never passes the poles", () => {
    const p = panBy(view, size, 100, 0);
    expect(p.centerLon).toBeLessThan(view.centerLon);
    const up = panBy({ centerLon: 0, centerLat: 80, zoom: 4 }, size, 0, 100000);
    expect(up.centerLat).toBeLessThanOrEqual(MAX_LAT);
    expect(up.centerLat).toBeGreaterThan(50);
  });
  it("uses the backend's Mercator latitude limit and formula", () => {
    expect(MAX_LAT).toBeCloseTo((Math.atan(Math.sinh(Math.PI)) * 180) / Math.PI, 12);
    expect(project(0, MAX_LAT)[1]).toBeCloseTo(0, 12);
    // backend: y = (1 - ln(tan φ + sec φ) / π) / 2
    const phi = (41.9 * Math.PI) / 180;
    expect(project(12.5, 41.9)[1]).toBeCloseTo((1 - Math.log(Math.tan(phi) + 1 / Math.cos(phi)) / Math.PI) / 2, 12);
  });
  it("lists the world copies that intersect the viewport", () => {
    expect(worldCopies({ centerLon: 0, centerLat: 0, zoom: 4 }, size)).toEqual([0]);
    expect(worldCopies({ centerLon: 0, centerLat: 0, zoom: 1 }, size)).toEqual([-2, -1, 0, 1, 2]);
    // centred near the antimeridian: the copy east of it is visible too
    expect(worldCopies({ centerLon: 179, centerLat: 0, zoom: 4 }, size)).toEqual([0, 1]);
  });
});
