import { describe, expect, it } from "vitest";
import type { ReviewShotView, ReviewVariantView } from "../../lib/ipc";
import { buildExportItems, countAtLeast, countByRating, countNotExportable } from "./export";

const variant = (
  preset: string,
  key: string,
  over: Partial<ReviewVariantView> = {},
): ReviewVariantView => ({
  preset,
  fileName: `${key}.jpg`,
  extension: "jpg",
  bytes: 1,
  kind: "jpeg" as ReviewVariantView["kind"],
  selectable: true,
  exportable: true,
  ...over,
});
const shots: ReviewShotView[] = ["s1", "s2", "s3", "s4"].map((key) => ({
  key,
  displayName: key,
  variants: [variant("a", key), variant("b", key)],
}));
const decision = (key: string, rating: number, winnerPreset: string | null) => ({
  key,
  rating,
  winnerPreset,
  reviewed: true,
});
const decisions = {
  s1: decision("s1", 5, "a"),
  s2: decision("s2", 3, "b"),
  s3: decision("s3", 0, "a"),
  s4: decision("s4", 4, null),
};

describe("buildExportItems", () => {
  it("esporta ogni scelta senza filtro di valutazione, saltando gli scatti senza scelta", () => {
    expect(buildExportItems(shots, decisions, 0).map((i) => i.outputName)).toEqual([
      "s1.jpg",
      "s2.jpg",
      "s3.jpg",
    ]);
  });

  it("tiene solo le scelte con almeno la valutazione minima, con scatto e preset", () => {
    expect(buildExportItems(shots, decisions, 3)).toEqual([
      { shotKey: "s1", preset: "a", outputName: "s1.jpg", rating: 5 },
      { shotKey: "s2", preset: "b", outputName: "s2.jpg", rating: 3 },
    ]);
    expect(buildExportItems(shots, decisions, 5)).toHaveLength(1);
  });

  it("ignora una scelta la cui variante non esiste più", () => {
    expect(buildExportItems(shots, { s1: decision("s1", 5, "zzz") }, 0)).toEqual([]);
  });

  it("esporta solo i JPEG: i TIFF e l'anteprima RAW restano fuori", () => {
    const mixed: ReviewShotView[] = [
      {
        key: "t",
        displayName: "t",
        variants: [variant("a", "t", { extension: "tif", exportable: false })],
      },
      { key: "j", displayName: "j", variants: [variant("a", "j")] },
    ];
    const picks = { t: decision("t", 4, "a"), j: decision("j", 4, "a") };
    expect(buildExportItems(mixed, picks, 0).map((i) => i.shotKey)).toEqual(["j"]);
    expect(countByRating(mixed, picks)).toEqual([0, 0, 0, 0, 1, 0]);
    expect(countNotExportable(mixed, picks)).toBe(1);
    expect(countNotExportable(mixed, picks, 5)).toBe(0);
  });
});

describe("countByRating", () => {
  it("conta le scelte per valutazione esatta e cumulativa", () => {
    const counts = countByRating(shots, decisions);
    expect(counts).toEqual([1, 0, 0, 1, 0, 1]);
    expect(countAtLeast(counts, 0)).toBe(3);
    expect(countAtLeast(counts, 4)).toBe(1);
  });
});
