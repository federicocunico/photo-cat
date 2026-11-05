import { describe, expect, it } from "vitest";
import { activeChips } from "./chips";

const t = (k: string, o?: object) =>
  o ? `${k}|${Object.values(o).join("|")}` : k;

describe("chip dei filtri di analisi", () => {
  it("mostra e rimuove stato di analisi, confidenza e dimensione dei gruppi", () => {
    const chips = activeChips(
      {
        analysis: { stage: "faces", state: "stale" },
        labelScore: { grp: "subject", value: "natura", min: 0.25, max: 0.35 },
        similarSize: "three_five",
      },
      t,
      undefined,
      undefined,
      (g) => g.toUpperCase(),
    );
    expect(chips.map((c) => c.name)).toEqual([
      "filterChips.analysis|filterChips.stage.faces|filterChips.state.stale",
      expect.stringContaining("SUBJECT"),
      "filterChips.similarSize.three_five",
    ]);
    expect(chips.map((c) => c.clear)).toEqual([
      { analysis: null },
      { labelScore: null },
      { similarSize: null },
    ]);
  });
});

describe("chip dell'area geografica", () => {
  it("mostra l'etichetta e si rimuove", () => {
    const chips = activeChips(
      { geoBbox: { south: 1, west: 2, north: 3, east: 4, approx: true, label: "Roma" } },
      t,
    );
    expect(chips.map((c) => c.name)).toEqual(["filterChips.area|Roma"]);
    expect(chips[0].clear).toEqual({ geoBbox: null });
  });
});
