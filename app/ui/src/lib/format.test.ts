import { describe, expect, it } from "vitest";
import { fmtEta } from "./format";

describe("fmtEta", () => {
  it("arrotonda per eccesso con precisione proporzionata alla durata", () => {
    expect(fmtEta(0)).toBe("5 s");
    expect(fmtEta(42)).toBe("45 s");
    expect(fmtEta(61)).toBe("1 min 10 s");
    expect(fmtEta(299)).toBe("5 min 00 s");
    expect(fmtEta(601)).toBe("11 min");
    expect(fmtEta(3 * 3600 + 5)).toBe("3 h 01 min");
  });

  it("non cambia a ogni secondo durante il conto alla rovescia", () => {
    const labels = new Set(Array.from({ length: 60 }, (_, i) => fmtEta(1260 - i)));
    expect(labels.size).toBe(1);
  });
});
