// Valori iniziali dell'editor della data di scatto, condivisi da Inspector e menu contestuale
// (bulk-date-inspector D3): con più foto si parte dalla data manuale comune, se c'è,
// altrimenti da "giorno" con i campi vuoti.
import type { DatePrecision, MediaDates, MediaDetail } from "../../lib/ipc";

/** Data impostata a mano: `value` è "AAAA", "AAAA-MM" o "AAAA-MM-GG". */
export type ManualDate = { value: string; precision: DatePrecision };

/** Stato dei campi dell'editor; i campi sono testo per poter restare vuoti. */
export type DateFields = { precision: DatePrecision; year: string; month: string; day: string };

const SLICE: Record<DatePrecision, number> = { year: 4, month: 7, day: 10 };

const EMPTY: DateFields = { precision: "day", year: "", month: "", day: "" };

/** Data manuale di una foto, o `null` se la data è quella del file. */
export function manualDateOf(dates: MediaDates | null | undefined): ManualDate | null {
  if (!dates?.manualAt || !dates.manualPrecision) return null;
  return {
    value: dates.manualAt.slice(0, SLICE[dates.manualPrecision]),
    precision: dates.manualPrecision,
  };
}

/**
 * Data manuale comune a tutte le foto (stesso periodo e stessa precisione), o `null`
 * se manca anche solo un dettaglio o una foto ha una data diversa.
 */
export function commonManualDate(details: (MediaDetail | undefined)[]): ManualDate | null {
  if (!details.length || details.some((d) => d == null)) return null;
  const dates = details.map((d) => manualDateOf(d!.dates));
  const first = dates[0];
  if (!first) return null;
  return dates.every((d) => d?.value === first.value && d.precision === first.precision)
    ? first
    : null;
}

/**
 * Campi iniziali: con una data manuale comune quella data e la sua precisione; altrimenti,
 * con più foto, "giorno" e campi vuoti (nessuna data precompilata da confermare per
 * sbaglio su tutto il gruppo); con una sola foto `single`, il comportamento di sempre.
 */
export function initialDateState(
  count: number,
  common: ManualDate | null,
  single: DateFields,
): DateFields {
  const base = count > 1 ? EMPTY : single;
  if (!common) return base;
  const [year = "", month = base.month, day = base.day] = common.value.split("-");
  return { precision: common.precision, year, month, day };
}
