import { describe, expect, it } from "vitest";
import { contrastRatio } from "./contrast";
import { applyTheme, contrastPairs, themes, type ThemeName } from "./tokens";

describe("design token", () => {
  for (const name of Object.keys(themes) as ThemeName[]) {
    it(`tema ${name}: tutte le coppie testo/sfondo hanno contrasto >= 4.5:1`, () => {
      const t = themes[name];
      const failures = contrastPairs
        .map(([fg, bg]) => ({ fg, bg, ratio: contrastRatio(t[fg], t[bg]) }))
        .filter((p) => p.ratio < 4.5)
        .map((p) => `${p.fg} su ${p.bg}: ${p.ratio.toFixed(2)}`);
      expect(failures).toEqual([]);
    });
  }

  it("i due temi definiscono gli stessi token", () => {
    expect(Object.keys(themes.light).sort()).toEqual(Object.keys(themes.dark).sort());
  });

  it("applyTheme scrive le custom properties", () => {
    applyTheme("light");
    const s = document.documentElement.style;
    expect(s.getPropertyValue("--pc-bg")).toBe(themes.light.bg);
    expect(s.getPropertyValue("--pc-text-secondary")).toBe(themes.light.textSecondary);
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("calcolo del contrasto WCAG", () => {
    expect(contrastRatio("#000000", "#FFFFFF")).toBeCloseTo(21, 0);
    expect(contrastRatio("#777777", "#777777")).toBeCloseTo(1, 5);
  });
});
