import { describe, expect, it } from "vitest";
import { datePath, selectedOf, toggleDate, toggleIn } from "./facets";

const none = { dateFrom: null, dateTo: null };
const year = { dateFrom: "2023-01-01", dateTo: "2023-12-31" };
const may = { dateFrom: "2023-05-01", dateTo: "2023-05-31" };
const day = { dateFrom: "2023-05-14", dateTo: "2023-05-14" };

describe("datePath", () => {
  it("riconosce anno, mese e giorno dall'intervallo", () => {
    expect(datePath(none)).toEqual({ year: null, month: null, day: null });
    expect(datePath(year)).toEqual({ year: "2023", month: null, day: null });
    expect(datePath(may)).toEqual({ year: "2023", month: "2023-05", day: null });
    expect(datePath({ dateFrom: "2024-02-01", dateTo: "2024-02-29" }).month).toBe("2024-02");
    expect(datePath(day)).toEqual({ year: "2023", month: "2023-05", day: "2023-05-14" });
  });

  it("un intervallo qualsiasi vale come anno dell'inizio", () => {
    expect(datePath({ dateFrom: "2023-05-03", dateTo: "2023-05-20" })).toEqual({
      year: "2023",
      month: null,
      day: null,
    });
    expect(datePath({ dateFrom: "2024-02-01", dateTo: "2024-02-28" }).month).toBeNull();
  });
});

describe("toggleDate", () => {
  it("scende nella gerarchia", () => {
    expect(toggleDate(none, "2023")).toEqual(year);
    expect(toggleDate(year, "2023-05")).toEqual(may);
    expect(toggleDate(may, "2023-05-14")).toEqual(day);
  });

  it("togliere la spunta risale di un livello", () => {
    expect(toggleDate(day, "2023-05-14")).toEqual(may);
    expect(toggleDate(may, "2023-05")).toEqual(year);
    expect(toggleDate(year, "2023")).toEqual(none);
  });

  it("togliere l'anno o il mese toglie anche i livelli sotto", () => {
    expect(toggleDate(day, "2023")).toEqual(none);
    expect(toggleDate(day, "2023-05")).toEqual(year);
  });

  it("scegliere un altro anno, mese o giorno sostituisce la scelta", () => {
    expect(toggleDate(may, "2022")).toEqual({ dateFrom: "2022-01-01", dateTo: "2022-12-31" });
    expect(toggleDate(day, "2023-03")).toEqual({ dateFrom: "2023-03-01", dateTo: "2023-03-31" });
    expect(toggleDate(day, "2023-05-20")).toEqual({ dateFrom: "2023-05-20", dateTo: "2023-05-20" });
  });

  it("la lista Anno usa la stessa logica", () => {
    const f = { ...may } as never;
    expect(selectedOf({ kind: "year" }, f)).toEqual(["2023"]);
    expect(toggleIn({ kind: "year" }, f, "2023")).toEqual(none);
    expect(toggleIn({ kind: "year" }, f, "2022")).toEqual({
      dateFrom: "2022-01-01",
      dateTo: "2022-12-31",
    });
  });
});
