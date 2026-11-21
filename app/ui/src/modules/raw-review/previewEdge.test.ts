import { describe, expect, it } from "vitest";
import { maxEdgeFor, previewEdge } from "./previewEdge";

describe("previewEdge", () => {
  it("usa il lato del precaricamento finché il pannello non ha bisogno di più pixel", () => {
    expect(previewEdge(640, 400, 1, 1)).toBe(2048);
    expect(previewEdge(1000, 900, 2, 1)).toBe(2048);
  });

  it("sale con la densità dello schermo, la dimensione del pannello e lo zoom", () => {
    expect(previewEdge(1600, 900, 2, 1)).toBe(4096);
    expect(previewEdge(1000, 600, 1, 2.5)).toBe(4096);
    expect(previewEdge(1000, 600, 1, 4)).toBe(4096);
    expect(previewEdge(1000, 600, 1, 4.5)).toBe(8192);
    expect(previewEdge(4000, 3000, 2, 8)).toBe(8192);
  });

  it("ignora valori non validi", () => {
    expect(previewEdge(0, 0, 0, 0)).toBe(2048);
  });

  it("con molti pannelli si ferma a 4096 px per non esaurire la memoria delle immagini", () => {
    expect(maxEdgeFor(1)).toBe(8192);
    expect(maxEdgeFor(2)).toBe(8192);
    expect(maxEdgeFor(3)).toBe(4096);
    expect(maxEdgeFor(9)).toBe(4096);
    expect(previewEdge(1000, 600, 2, 8, maxEdgeFor(9))).toBe(4096);
    expect(previewEdge(640, 400, 1, 1, maxEdgeFor(9))).toBe(2048);
  });
});
