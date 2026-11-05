// Minimal TopoJSON decoder: quantized arcs -> lon/lat rings.

export interface Topology {
  type: "Topology";
  transform?: { scale: [number, number]; translate: [number, number] };
  arcs: number[][][];
  objects: Record<string, TopoGeometry>;
}
export interface TopoGeometry {
  type: string;
  arcs?: unknown;
  id?: string | number;
  properties?: Record<string, unknown>;
  geometries?: TopoGeometry[];
}

/** [lon, lat] */
export type LonLat = [number, number];
export type Ring = LonLat[];
/** A polygon: outer ring first, then holes. */
export type Polygon = Ring[];
export interface Country {
  id?: string | number;
  name?: string;
  polygons: Polygon[];
}

/** Decode all arcs to absolute lon/lat coordinates. */
export function decodeArcs(topo: Topology): LonLat[][] {
  const t = topo.transform;
  return topo.arcs.map((arc) => {
    let x = 0;
    let y = 0;
    return arc.map(([dx, dy]) => {
      if (t) {
        x += dx;
        y += dy;
        return [x * t.scale[0] + t.translate[0], y * t.scale[1] + t.translate[1]] as LonLat;
      }
      return [dx, dy] as LonLat;
    });
  });
}

function stitch(arcIdx: number[], arcs: LonLat[][]): Ring {
  const ring: Ring = [];
  for (const i of arcIdx) {
    const pts = i < 0 ? arcs[~i].slice().reverse() : arcs[i];
    // consecutive arcs share an endpoint: drop the duplicate
    for (let k = ring.length ? 1 : 0; k < pts.length; k++) ring.push(pts[k]);
  }
  return ring;
}

function geometryPolygons(g: TopoGeometry, arcs: LonLat[][]): Polygon[] {
  if (g.type === "Polygon") {
    return [(g.arcs as number[][]).map((r) => stitch(r, arcs))];
  }
  if (g.type === "MultiPolygon") {
    return (g.arcs as number[][][]).map((p) => p.map((r) => stitch(r, arcs)));
  }
  return [];
}

/** Assemble the geometries of a TopoJSON object into countries. */
export function decodeObject(topo: Topology, objectName: string): Country[] {
  const obj = topo.objects[objectName];
  if (!obj) return [];
  const arcs = decodeArcs(topo);
  const geoms = obj.type === "GeometryCollection" ? (obj.geometries ?? []) : [obj];
  return geoms.map((g) => ({
    id: g.id,
    name: typeof g.properties?.name === "string" ? g.properties.name : undefined,
    polygons: geometryPolygons(g, arcs),
  }));
}
