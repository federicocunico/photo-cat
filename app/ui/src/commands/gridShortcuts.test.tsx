import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { useActiveGrid, useGridShortcuts, type ActiveGrid } from "./gridShortcuts";
import { useCommandStore } from "./registry";

afterEach(() => {
  cleanup();
  useCommandStore.setState({ commands: [] });
});

function Host({ grid }: { grid: ActiveGrid }) {
  useGridShortcuts();
  useActiveGrid(grid);
  return (
    <>
      <input aria-label="Cerca" />
      <div id="area" />
    </>
  );
}

const mk = (count = 3): ActiveGrid => ({
  selectAll: vi.fn(),
  invert: vi.fn(),
  clear: vi.fn(),
  count: () => count,
});

describe("scorciatoie di selezione della shell", () => {
  it("Ctrl+A, Ctrl+Maiusc+I ed Esc vanno alla griglia attiva", () => {
    const grid = mk();
    render(<Host grid={grid} />);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    fireEvent.keyDown(window, { key: "I", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(window, { key: "Escape" });
    expect(grid.selectAll).toHaveBeenCalledTimes(1);
    expect(grid.invert).toHaveBeenCalledTimes(1);
    expect(grid.clear).toHaveBeenCalledTimes(1);
  });

  it("Cmd+A funziona su macOS", () => {
    const grid = mk();
    render(<Host grid={grid} />);
    fireEvent.keyDown(window, { key: "a", metaKey: true });
    expect(grid.selectAll).toHaveBeenCalledTimes(1);
  });

  it("nel campo di ricerca le scorciatoie sono ignorate", () => {
    const grid = mk();
    const { getByLabelText } = render(<Host grid={grid} />);
    const input = getByLabelText("Cerca");
    input.focus();
    fireEvent.keyDown(input, { key: "a", ctrlKey: true });
    fireEvent.keyDown(input, { key: "I", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(grid.selectAll).not.toHaveBeenCalled();
    expect(grid.invert).not.toHaveBeenCalled();
    expect(grid.clear).not.toHaveBeenCalled();
  });

  it("con un dialogo aperto sono ignorate", () => {
    const grid = mk();
    render(<Host grid={grid} />);
    const dlg = document.createElement("div");
    dlg.setAttribute("role", "dialog");
    dlg.setAttribute("aria-modal", "true");
    document.body.appendChild(dlg);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    fireEvent.keyDown(window, { key: "Escape" });
    expect(grid.selectAll).not.toHaveBeenCalled();
    expect(grid.clear).not.toHaveBeenCalled();
    dlg.remove();
  });

  it("con il visore aperto Ctrl+A e Ctrl+Maiusc+I non toccano la griglia sotto", () => {
    const grid = mk();
    render(<Host grid={grid} />);
    // Il visore è un dialogo Radix senza aria-modal: il registro non lo vede come modale.
    const viewer = document.createElement("div");
    viewer.setAttribute("role", "dialog");
    viewer.className = "viewer";
    document.body.appendChild(viewer);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    fireEvent.keyDown(window, { key: "I", ctrlKey: true, shiftKey: true });
    expect(grid.selectAll).not.toHaveBeenCalled();
    expect(grid.invert).not.toHaveBeenCalled();
    viewer.remove();
    // Un pannello flottante non modale non blocca le scorciatoie.
    const panel = document.createElement("div");
    panel.setAttribute("role", "dialog");
    panel.setAttribute("aria-modal", "false");
    document.body.appendChild(panel);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(grid.selectAll).toHaveBeenCalledTimes(1);
    panel.remove();
  });

  it("Esc non fa nulla senza selezione e senza griglia attiva", () => {
    const grid = mk(0);
    const { unmount } = render(<Host grid={grid} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(grid.clear).not.toHaveBeenCalled();
    unmount();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(grid.selectAll).not.toHaveBeenCalled();
  });
});
