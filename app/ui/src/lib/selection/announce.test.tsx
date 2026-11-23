import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import "../../i18n";
import { SelectionHarness } from "../../test/selectionHarness";

afterEach(cleanup);

const ids = [1, 2, 3, 4, 5, 6];

describe("menu contestuale e annunci della selezione", () => {
  it("clic destro su un elemento selezionato tiene tutta la selezione", () => {
    render(<SelectionHarness ids={ids} layout="grid" />);
    const cells = screen.getAllByRole("gridcell");
    fireEvent.click(cells[0]!);
    fireEvent.click(cells[4]!, { shiftKey: true });
    fireEvent.contextMenu(cells[2]!);
    expect(screen.getByRole("menu", { name: "Azioni su 5 elementi" })).toBeInTheDocument();
    expect(cells.filter((c) => c.getAttribute("aria-selected") === "true")).toHaveLength(5);
  });

  it("clic destro su un elemento non selezionato seleziona solo lui", () => {
    render(<SelectionHarness ids={ids} layout="grid" />);
    const cells = screen.getAllByRole("gridcell");
    fireEvent.click(cells[0]!);
    fireEvent.click(cells[4]!, { shiftKey: true });
    fireEvent.contextMenu(cells[5]!);
    expect(screen.getByRole("menu", { name: "Azioni su 1 elementi" })).toBeInTheDocument();
    expect(
      cells.filter((c) => c.getAttribute("aria-selected") === "true").map((c) => c.dataset.id),
    ).toEqual(["6"]);
  });

  it("annuncia il numero di selezionati a ogni cambiamento", () => {
    render(<SelectionHarness ids={ids} layout="grid" />);
    const status = screen.getByRole("status");
    const grid = screen.getByRole("grid");
    act(() => {
      fireEvent.keyDown(grid, { key: "End" });
    });
    expect(status).toHaveTextContent("1 selezionata");
    act(() => {
      fireEvent.keyDown(grid, { key: "ArrowLeft", shiftKey: true });
    });
    expect(status).toHaveTextContent("2 selezionate");
    act(() => {
      fireEvent.keyDown(grid, { key: "Escape" });
    });
    expect(status).toHaveTextContent("Nessun elemento selezionato");
  });
});
