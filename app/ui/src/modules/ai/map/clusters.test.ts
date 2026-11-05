import { describe, expect, it } from "vitest";
import type { Cell } from "../../../bindings";
import { lonLatToPixel } from "../../../components/map/projection";
import {
  MAX_RADIUS,
  MIN_RADIUS,
  cellPx,
  gridZoom,
  clusterLabel,
  hitTest,
  isApproxCluster,
  layoutClusters,
  radiusFor,
} from "./clusters";

const cell = (over: Partial<Cell>): Cell => ({
  cx: 0,
  cy: 0,
  bbox: { south: -1, west: -1, north: 1, east: 1 },
  manual: false,
  sampleThumbs: [],
  count: 1,
  approxCount: 0,
  lat: 0,
  lon: 0,
  place: null,
  sampleIds: [],
  ...over,
});
const view = { centerLon: 0, centerLat: 0, zoom: 2 };
const size = { width: 800, height: 600 };
const proj = (lon: number, lat: number) => lonLatToPixel(view, size, lon, lat);

describe("radiusFor / gridZoom", () => {
  it("grows with sqrt(count) within the limits", () => {
    expect(radiusFor(0)).toBe(MIN_RADIUS);
    expect(radiusFor(1_000_000)).toBe(MAX_RADIUS);
    expect(radiusFor(100)).toBeGreaterThan(radiusFor(25));
    expect(radiusFor(100) / radiusFor(25)).toBeLessThan(2);
  });
  it("maps the view zoom to a grid level", () => {
    expect(gridZoom(1)).toBe(0);
    expect(gridZoom(2)).toBe(1);
    expect(gridZoom(64)).toBe(6);
  });
  it("keeps cells at least ~24 px wide", () => {
    for (const z of [1, 1.5, 3, 7, 20, 64]) expect(cellPx(z, gridZoom(z))).toBeGreaterThanOrEqual(24 - 1e-6);
  });
});

describe("layoutClusters", () => {
  it("keeps distant cells apart and skips cells without coordinates", () => {
    const out = layoutClusters(
      [cell({ lon: -60, lat: 0, count: 5 }), cell({ lon: 60, lat: 0, count: 7 }), cell({ lat: null, lon: null })],
      proj,
    );
    expect(out).toHaveLength(2);
    expect(out[0].count).toBe(7);
  });
  it("sizes circles so neighbouring cells never overlap", () => {
    const px = cellPx(2, 1); // 32 * 2 / 2
    const out = layoutClusters(
      [cell({ lon: 0, lat: 0, count: 100000 }), cell({ lon: 22.5, lat: 0, count: 100000 })],
      proj,
      px,
    );
    expect(out).toHaveLength(2);
    for (const c of out) expect(c.r).toBeLessThanOrEqual(0.45 * px + 1e-9);
    expect(Math.hypot(out[0].x - out[1].x, out[0].y - out[1].y)).toBeGreaterThan(out[0].r + out[1].r);
  });
  it("flags clusters made mostly of approximate photos", () => {
    expect(isApproxCluster({ count: 10, approxCount: 6 })).toBe(true);
    expect(isApproxCluster({ count: 10, approxCount: 5 })).toBe(false);
  });
});

describe("hitTest", () => {
  it("returns the circle containing the point, or null", () => {
    const [far] = layoutClusters([cell({ lon: 50, lat: 0, count: 4 })], proj);
    expect(hitTest([far], far.x + 1, far.y)).toBe(far);
    expect(hitTest([far], far.x + far.r + 5, far.y)).toBeNull();
  });
});

describe("clusterLabel", () => {
  it("uses the place, else the coordinates", () => {
    const [c] = layoutClusters([cell({ lon: 0, lat: 0, count: 3 })], proj);
    expect(clusterLabel(c)).toBe("0.00, 0.00");
    expect(clusterLabel({ ...c, place: "Roma" })).toBe("Roma");
  });
});
