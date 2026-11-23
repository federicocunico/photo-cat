import { describe, expect, it } from "vitest";
import {
  clickSelect,
  idsSelection,
  selCount,
  selectAllOf,
  selectForContextMenu,
  selHas,
  selSet,
} from "./model";

describe("modello di selezione", () => {
  it("clickSelect funziona con id testuali", () => {
    const order = ["a", "b", "c", "d"];
    expect(clickSelect(order, ["a"], "a", "c", { ctrl: false, shift: true })).toEqual([
      "a",
      "b",
      "c",
    ]);
    expect(clickSelect(order, ["a"], "a", "d", { ctrl: true, shift: false })).toEqual(["a", "d"]);
  });

  it("una selezione per filtro tratta gli id testuali senza limite di maxId", () => {
    const s = selSet(selectAllOf<null, string>(null, { count: 3, maxId: 0 }), ["b"], false);
    expect(selHas(s, "a")).toBe(true);
    expect(selHas(s, "b")).toBe(false);
    expect(selCount(s)).toBe(2);
  });
});

describe("selectForContextMenu", () => {
  it("clic destro su un elemento selezionato: la selezione resta (stessa istanza)", () => {
    const s = idsSelection<null>([1, 2, 3, 4, 5]);
    expect(selectForContextMenu(s, 3)).toBe(s);
  });

  it("clic destro su un elemento non selezionato: resta selezionato solo lui", () => {
    const s = idsSelection<null>([1, 2, 3, 4, 5]);
    const next = selectForContextMenu(s, 6);
    expect(selCount(next)).toBe(1);
    expect(selHas(next, 6)).toBe(true);
    expect(selHas(next, 1)).toBe(false);
  });

  it("vale anche per una selezione per filtro", () => {
    const all = selectAllOf<null>(null, { count: 100, maxId: 100 });
    expect(selectForContextMenu(all, 40)).toBe(all);
    const without40 = selSet(all, [40], false);
    expect(selCount(selectForContextMenu(without40, 40))).toBe(1);
  });
});
