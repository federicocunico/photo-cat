import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { useRef } from "react";
import { useActivePane, useGridShortcuts, type ActivePane } from "./gridShortcuts";
import { useCommandStore } from "./registry";

afterEach(() => {
  cleanup();
  useCommandStore.setState({ commands: [] });
});

function Host({ grid }: { grid: ActivePane }) {
  useGridShortcuts();
  useActivePane(grid);
  return (
    <>
      <input aria-label="Cerca" />
      <div id="area" />
    </>
  );
}

const mk = (count = 3): ActivePane => ({
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

  it("il riquadro attivo è quello dell'ultimo clic o focus, in qualunque ordine di montaggio", () => {
    const list = mk();
    const faces = mk();
    function Pane({ grid, name }: { grid: ActivePane; name: string }) {
      const ref = useRef<HTMLDivElement>(null);
      useActivePane(grid, ref);
      return (
        <div ref={ref} data-testid={name}>
          <button type="button">{name}</button>
        </div>
      );
    }
    function Pair({ listFirst }: { listFirst: boolean }) {
      useGridShortcuts();
      return listFirst ? (
        <>
          <Pane grid={list} name="elenco" />
          <Pane grid={faces} name="volti" />
        </>
      ) : (
        <>
          <Pane grid={faces} name="volti" />
          <Pane grid={list} name="elenco" />
        </>
      );
    }
    for (const listFirst of [true, false]) {
      const { unmount, getByTestId, getByText } = render(<Pair listFirst={listFirst} />);
      // Senza interazioni vale l'ultimo montato.
      fireEvent.keyDown(window, { key: "a", ctrlKey: true });
      expect((listFirst ? faces : list).selectAll).toHaveBeenCalledTimes(1);
      fireEvent.mouseDown(getByText("elenco"));
      fireEvent.keyDown(window, { key: "a", ctrlKey: true });
      expect(list.selectAll).toHaveBeenCalledTimes(listFirst ? 1 : 2);
      expect(getByTestId("elenco")).toHaveAttribute("data-active-pane");
      expect(getByTestId("volti")).not.toHaveAttribute("data-active-pane");
      getByText("volti").focus();
      fireEvent.keyDown(window, { key: "a", ctrlKey: true });
      expect(faces.selectAll).toHaveBeenCalledTimes(listFirst ? 2 : 1);
      expect(getByTestId("volti")).toHaveAttribute("data-active-pane");
      expect(getByTestId("elenco")).not.toHaveAttribute("data-active-pane");
      // Ogni riquadro conserva la propria selezione: il cambio di riquadro non svuota l'altro.
      expect(list.clear).not.toHaveBeenCalled();
      expect(faces.clear).not.toHaveBeenCalled();
      unmount();
      useCommandStore.setState({ commands: [] });
      vi.mocked(list.selectAll).mockClear();
      vi.mocked(faces.selectAll).mockClear();
    }
  });

  it("un clic fuori dai riquadri (es. in un dialogo) non cambia il riquadro attivo", () => {
    const list = mk();
    const faces = mk();
    function Pane({ grid, name }: { grid: ActivePane; name: string }) {
      const ref = useRef<HTMLDivElement>(null);
      useActivePane(grid, ref);
      return <div ref={ref}>{name}</div>;
    }
    function Pair() {
      useGridShortcuts();
      return (
        <>
          <Pane grid={list} name="elenco" />
          <Pane grid={faces} name="volti" />
        </>
      );
    }
    const { getByText } = render(<Pair />);
    fireEvent.mouseDown(getByText("elenco"));
    const portal = document.createElement("div");
    document.body.appendChild(portal);
    fireEvent.mouseDown(portal);
    portal.remove();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(list.selectAll).toHaveBeenCalledTimes(1);
    expect(faces.selectAll).not.toHaveBeenCalled();
  });

  it("un solo riquadro non riceve l'indicatore", () => {
    const grid = mk();
    function One() {
      const ref = useRef<HTMLDivElement>(null);
      useGridShortcuts();
      useActivePane(grid, ref);
      return <div ref={ref} data-testid="solo" />;
    }
    const { getByTestId } = render(<One />);
    fireEvent.mouseDown(getByTestId("solo"));
    expect(getByTestId("solo")).not.toHaveAttribute("data-active-pane");
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
