import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { navigationTarget } from "./useSelectionKeys";
import { SelectionHarness } from "../../test/selectionHarness";

const ids = Array.from({ length: 12 }, (_, i) => i + 1);

afterEach(cleanup);

function setup(layout: "grid" | "list" = "grid") {
  render(<SelectionHarness ids={ids} layout={layout} columns={4} pageSize={8} />);
  const box = screen.getByRole(layout === "grid" ? "grid" : "listbox");
  const selected = () =>
    screen
      .getAllByRole(layout === "grid" ? "gridcell" : "option")
      .filter((el) => el.getAttribute("aria-selected") === "true")
      .map((el) => Number(el.dataset.id));
  const focused = () => Number(box.getAttribute("aria-activedescendant")?.replace("item-", ""));
  const key = (
    k: string,
    mods: Partial<Record<"ctrlKey" | "shiftKey" | "metaKey", boolean>> = {},
  ) =>
    act(() => {
      fireEvent.keyDown(box, { key: k, ...mods });
    });
  return { box, selected, focused, key };
}

describe("navigationTarget", () => {
  const base = { layout: "grid" as const, columns: 4, pageSize: 8, count: 12 };
  it("in griglia muove su righe e colonne, senza uscire dai bordi in verticale", () => {
    expect(navigationTarget("ArrowRight", { ...base, cursor: 0 })).toBe(1);
    expect(navigationTarget("ArrowDown", { ...base, cursor: 1 })).toBe(5);
    expect(navigationTarget("ArrowUp", { ...base, cursor: 1 })).toBe(1);
    expect(navigationTarget("ArrowDown", { ...base, cursor: 9 })).toBe(9);
    expect(navigationTarget("End", { ...base, cursor: 0 })).toBe(11);
    expect(navigationTarget("PageDown", { ...base, cursor: 2 })).toBe(10);
  });
  it("in un elenco ignora destra e sinistra", () => {
    expect(navigationTarget("ArrowRight", { ...base, layout: "list", cursor: 0 })).toBeNull();
    expect(navigationTarget("ArrowDown", { ...base, layout: "list", cursor: 0 })).toBe(1);
  });
  it("senza focus parte dal primo elemento (o dall'ultimo con freccia su)", () => {
    expect(navigationTarget("ArrowDown", { ...base, cursor: -1 })).toBe(0);
    expect(navigationTarget("ArrowUp", { ...base, cursor: -1 })).toBe(11);
  });
});

describe("useSelectionKeys", () => {
  it("le frecce spostano il focus e selezionano solo l'elemento raggiunto", () => {
    const t = setup();
    t.key("ArrowRight");
    expect(t.selected()).toEqual([1]);
    t.key("ArrowRight");
    expect(t.selected()).toEqual([2]);
    t.key("ArrowDown");
    expect(t.selected()).toEqual([6]);
    expect(t.focused()).toBe(6);
  });

  it("Maiusc estende dall'ancora, Ctrl+Maiusc aggiunge l'intervallo", () => {
    const t = setup();
    t.key("ArrowRight"); // 1
    t.key("ArrowRight", { shiftKey: true });
    t.key("ArrowRight", { shiftKey: true });
    expect(t.selected()).toEqual([1, 2, 3]);
    t.key("ArrowLeft", { shiftKey: true });
    expect(t.selected()).toEqual([1, 2]);
  });

  it("Ctrl sposta solo il focus e Ctrl+Spazio aggiunge o toglie", () => {
    const t = setup();
    t.key("ArrowRight"); // 1
    t.key("ArrowRight", { ctrlKey: true });
    t.key("ArrowRight", { ctrlKey: true });
    t.key("ArrowRight", { ctrlKey: true });
    expect(t.selected()).toEqual([1]);
    expect(t.focused()).toBe(4);
    t.key(" ", { ctrlKey: true });
    expect(t.selected()).toEqual([1, 4]);
    t.key(" ", { ctrlKey: true });
    expect(t.selected()).toEqual([1]);
  });

  it("Home, Fine e Pagina giù selezionano l'elemento raggiunto", () => {
    const t = setup();
    t.key("End");
    expect(t.selected()).toEqual([12]);
    t.key("Home");
    expect(t.selected()).toEqual([1]);
    t.key("PageDown");
    expect(t.selected()).toEqual([9]);
    t.key("Home", { shiftKey: true });
    expect(t.selected()).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  it("Ctrl+A, Ctrl+Maiusc+I ed Esc", async () => {
    const t = setup();
    t.key("ArrowRight");
    t.key("a", { ctrlKey: true });
    await act(async () => {});
    expect(t.selected()).toHaveLength(12);
    t.key("Escape");
    expect(t.selected()).toEqual([]);
    t.key("ArrowRight"); // 2 (il focus era su 1)
    t.key("I", { ctrlKey: true, shiftKey: true });
    await act(async () => {});
    expect(t.selected()).toHaveLength(11);
  });

  it("in un elenco su e giù selezionano, destra e sinistra no", () => {
    const t = setup("list");
    t.key("ArrowDown");
    t.key("ArrowDown");
    expect(t.selected()).toEqual([2]);
    t.key("ArrowRight");
    expect(t.selected()).toEqual([2]);
    t.key("ArrowDown", { shiftKey: true });
    t.key("ArrowDown", { shiftKey: true });
    expect(t.selected()).toEqual([2, 3, 4]);
  });

  it("nessun effetto quando il focus è in un campo di testo dentro la raccolta", () => {
    render(<SelectionHarness ids={ids} layout="grid" columns={4} pageSize={8} withInput />);
    const grid = screen.getByRole("grid");
    act(() => {
      fireEvent.keyDown(grid, { key: "ArrowRight" });
    });
    const input = screen.getByRole("textbox", { name: "rinomina" });
    act(() => {
      fireEvent.keyDown(input, { key: "a", ctrlKey: true });
      fireEvent.keyDown(input, { key: "Escape" });
      fireEvent.keyDown(input, { key: "ArrowRight" });
    });
    // I tasti nel campo bubblano alla griglia, ma il gestore li ignora: la selezione resta.
    expect(
      screen.getAllByRole("gridcell").filter((el) => el.getAttribute("aria-selected") === "true"),
    ).toHaveLength(1);
    expect(grid.getAttribute("aria-activedescendant")).toBe("item-1");
  });
});
