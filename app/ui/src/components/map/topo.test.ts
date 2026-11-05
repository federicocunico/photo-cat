import { describe, expect, it } from "vitest";
import { decodeArcs, decodeObject, type Topology } from "./topo";

const topo: Topology = {
  type: "Topology",
  transform: { scale: [1, 1], translate: [10, 20] },
  // arc0: (0,0)->(2,0)->(2,2) ; arc1: (2,2)->(0,2)->(0,0)
  arcs: [
    [[0, 0], [2, 0], [0, 2]],
    [[2, 2], [-2, 0], [0, -2]],
  ],
  objects: {
    c: {
      type: "GeometryCollection",
      geometries: [
        { type: "Polygon", id: "A", properties: { name: "Alpha" }, arcs: [[0, 1]] },
        { type: "MultiPolygon", id: "B", arcs: [[[~1, ~0]]] },
      ],
    },
  },
};

describe("topo", () => {
  it("delta-decodes and applies the transform", () => {
    const arcs = decodeArcs(topo);
    expect(arcs[0]).toEqual([[10, 20], [12, 20], [12, 22]]);
    expect(arcs[1]).toEqual([[12, 22], [10, 22], [10, 20]]);
  });

  it("stitches rings, dropping shared endpoints", () => {
    const [a] = decodeObject(topo, "c");
    expect(a.name).toBe("Alpha");
    expect(a.polygons[0][0]).toEqual([[10, 20], [12, 20], [12, 22], [10, 22], [10, 20]]);
  });

  it("reverses negative arc indices", () => {
    const [, b] = decodeObject(topo, "c");
    expect(b.polygons[0][0]).toEqual([[10, 20], [10, 22], [12, 22], [12, 20], [10, 20]]);
  });

  it("returns [] for an unknown object", () => {
    expect(decodeObject(topo, "nope")).toEqual([]);
  });
});
