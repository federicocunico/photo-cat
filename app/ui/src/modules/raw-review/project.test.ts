import { describe, expect, it } from "vitest";
import {
  MAX_PANELS,
  RAW_PREVIEW,
  movePanel,
  panelCandidates,
  panelGrid,
  panelLock,
  showAllPanels,
  togglePanel,
} from "./project";

const presets = ["a", "b", "c", "d"];
const base = () => ({ panelPresets: [...presets], hiddenPresets: [] as string[] });

describe("panelCandidates", () => {
  it("mette la guida per prima e, con i RAF, l'anteprima RAW in fondo", () => {
    expect(panelCandidates("presets", ["a", "b", "c"], "c")).toEqual(["c", "a", "b"]);
    expect(panelCandidates("rafOnly", ["a"], "a")).toEqual(["a", RAW_PREVIEW]);
    expect(panelCandidates("rafWithVariants", [], null)).toEqual([RAW_PREVIEW]);
  });
});

describe("visibilità dei pannelli", () => {
  it("nasconde e mostra un preset, rimettendolo in fondo", () => {
    const hidden = togglePanel(base(), "b", presets);
    expect(hidden.panelPresets).toEqual(["a", "c", "d"]);
    expect(hidden.hiddenPresets).toEqual(["b"]);
    const shown = togglePanel(hidden, "b", presets);
    expect(shown.panelPresets).toEqual(["a", "c", "d", "b"]);
    expect(shown.hiddenPresets).toEqual([]);
  });

  it("lascia sempre almeno un pannello visibile", () => {
    let state = base();
    for (const preset of presets) state = togglePanel(state, preset, presets);
    expect(state.panelPresets).toEqual(["d"]);
    expect(panelLock(state.panelPresets, "d")).toBe("last");
  });

  it("non supera il massimo di pannelli e ignora i preset sconosciuti", () => {
    const many = Array.from({ length: 12 }, (_, i) => `p${i}`);
    const full = { panelPresets: many.slice(0, MAX_PANELS), hiddenPresets: many.slice(MAX_PANELS) };
    expect(panelLock(full.panelPresets, "p9")).toBe("max");
    expect(togglePanel(full, "p9", many)).toBe(full);
    const fewer = togglePanel(base(), "a", presets);
    expect(togglePanel(fewer, "zzz", presets)).toBe(fewer);
  });

  it("mostra di nuovo tutti i preset senza cambiare l'ordine corrente", () => {
    const start = { panelPresets: ["c", "a", "b", "d"], hiddenPresets: [] };
    const state = showAllPanels(togglePanel(start, "b", presets), presets);
    expect(state.panelPresets).toEqual(["c", "a", "d", "b"]);
    expect(state.hiddenPresets).toEqual([]);
  });
});

describe("ordine dei pannelli", () => {
  it("sposta un pannello senza cambiare quali sono visibili", () => {
    expect(movePanel(base(), "d", 0).panelPresets).toEqual(["d", "a", "b", "c"]);
    expect(movePanel(base(), "a", 3).panelPresets).toEqual(["b", "c", "d", "a"]);
    const hidden = togglePanel(base(), "b", presets);
    const moved = movePanel(hidden, "d", 1);
    expect(moved.panelPresets).toEqual(["a", "d", "c"]);
    expect(moved.hiddenPresets).toEqual(["b"]);
  });

  it("ignora spostamenti non validi", () => {
    const state = base();
    expect(movePanel(state, "zzz", 0)).toBe(state);
    expect(movePanel(state, "a", 4)).toBe(state);
    expect(movePanel(state, "a", -1)).toBe(state);
    expect(movePanel(state, "a", 0)).toBe(state);
  });
});

describe("panelGrid", () => {
  it("adatta la griglia al numero di pannelli, da 1 a 9", () => {
    expect([1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => panelGrid(n))).toEqual([
      { columns: 1, rows: 1 },
      { columns: 2, rows: 1 },
      { columns: 3, rows: 1 },
      { columns: 2, rows: 2 },
      { columns: 3, rows: 2 },
      { columns: 3, rows: 2 },
      { columns: 3, rows: 3 },
      { columns: 3, rows: 3 },
      { columns: 3, rows: 3 },
    ]);
    expect(panelGrid(0)).toEqual({ columns: 1, rows: 1 });
  });
});
