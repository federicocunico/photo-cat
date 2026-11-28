import { expect, it } from "vitest";
import type { MediaDetail } from "../../lib/ipc";
import { commonManualDate, initialDateState, manualDateOf } from "./manualDate";

const SINGLE = { precision: "day", year: "2026", month: "11", day: "28" } as const;

it("con più foto e senza data comune: giorno e campi vuoti", () => {
  expect(initialDateState(60, null, SINGLE)).toEqual({
    precision: "day",
    year: "",
    month: "",
    day: "",
  });
});

it("con una sola foto: i valori di sempre", () => {
  expect(initialDateState(1, null, SINGLE)).toEqual(SINGLE);
});

it("con una data manuale comune: quella data e la sua precisione", () => {
  expect(initialDateState(5, { value: "1984-06", precision: "month" }, SINGLE)).toEqual({
    precision: "month",
    year: "1984",
    month: "06",
    day: "",
  });
  expect(initialDateState(1, { value: "1962", precision: "year" }, SINGLE)).toEqual({
    precision: "year",
    year: "1962",
    month: "11",
    day: "28",
  });
});

const withManual = (manualAt: string | null, manualPrecision: "day" | "month" | "year" | null) =>
  ({
    dates: { capturedAt: null, capturedSource: null, modifiedAt: null, manualAt, manualPrecision },
  }) as unknown as MediaDetail;

it("data manuale di una foto tagliata alla precisione", () => {
  expect(manualDateOf(withManual("1984-06-01 00:00:00", "month").dates)).toEqual({
    value: "1984-06",
    precision: "month",
  });
  expect(manualDateOf(withManual(null, null).dates)).toBeNull();
  expect(manualDateOf(undefined)).toBeNull();
});

it("data manuale comune solo se uguale per tutte e con tutti i dettagli", () => {
  const june = withManual("1984-06-01 00:00:00", "month");
  expect(commonManualDate([june, june])).toEqual({ value: "1984-06", precision: "month" });
  // Stesso istante, precisione diversa: non è la stessa data.
  expect(commonManualDate([june, withManual("1984-06-01 00:00:00", "day")])).toBeNull();
  expect(commonManualDate([june, withManual(null, null)])).toBeNull();
  expect(commonManualDate([june, undefined])).toBeNull();
  expect(commonManualDate([])).toBeNull();
});
