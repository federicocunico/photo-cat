import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { availableCommands, registerCommands, shortcutText, useCommands } from "./registry";
import { useCommandStore } from "./registry";

afterEach(() => {
  cleanup();
  useCommandStore.setState({ commands: [] });
});

const key = (k: string, opts: Partial<KeyboardEventInit> = {}, target: Window | Element = window) =>
  fireEvent.keyDown(target, { key: k, ...opts });

describe("registro dei comandi", () => {
  it("esegue la scorciatoia e rispetta la condizione", () => {
    const run = vi.fn();
    let ok = false;
    const off = registerCommands([
      {
        id: "x",
        label: "X",
        group: "general",
        shortcut: { key: "k", mod: true },
        when: () => ok,
        run,
      },
    ]);
    key("k", { ctrlKey: true });
    expect(run).not.toHaveBeenCalled();
    ok = true;
    key("k", { ctrlKey: true });
    expect(run).toHaveBeenCalledTimes(1);
    key("k");
    expect(run).toHaveBeenCalledTimes(1);
    off();
    key("k", { ctrlKey: true });
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("ignora i campi di testo, salvo i comandi che lo chiedono (Ctrl+K)", () => {
    const tag = vi.fn();
    const palette = vi.fn();
    registerCommands([
      { id: "tag", label: "Tag", group: "selection", shortcut: { key: "2" }, run: tag },
      {
        id: "palette",
        label: "Palette",
        group: "general",
        shortcut: { key: "k", mod: true },
        inInputs: true,
        run: palette,
      },
    ]);
    const input = document.createElement("input");
    document.body.appendChild(input);
    key("2", {}, input);
    expect(tag).not.toHaveBeenCalled();
    key("k", { ctrlKey: true }, input);
    expect(palette).toHaveBeenCalledTimes(1);
    input.remove();
  });

  it("a parità di tasti vince il comando registrato per ultimo", () => {
    const shell = vi.fn();
    const module = vi.fn();
    registerCommands([
      { id: "a", label: "A", group: "general", shortcut: { key: "a" }, run: shell },
    ]);
    registerCommands([
      { id: "b", label: "B", group: "sequence", shortcut: { key: "a" }, run: module },
    ]);
    key("a");
    expect(module).toHaveBeenCalledTimes(1);
    expect(shell).not.toHaveBeenCalled();
  });

  it("i simboli come ? non dipendono dal tasto Maiusc", () => {
    const run = vi.fn();
    registerCommands([{ id: "?", label: "Aiuto", group: "general", shortcut: { key: "?" }, run }]);
    key("?", { shiftKey: true });
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("useCommands registra finché il componente è montato; la palette vede quelli disponibili", () => {
    function Host({ on }: { on: boolean }) {
      useCommands([
        { id: "go", label: "Vai", group: "navigation", run: () => undefined },
        { id: "hidden", label: "Nascosto", group: "general", palette: false, run: () => undefined },
        { id: "off", label: "Spento", group: "general", when: () => on, run: () => undefined },
      ]);
      return null;
    }
    const view = render(<Host on={false} />);
    expect(availableCommands().map((c) => c.id)).toEqual(["go"]);
    view.rerender(<Host on={true} />);
    expect(availableCommands().map((c) => c.id)).toEqual(["go", "off"]);
    view.unmount();
    expect(availableCommands()).toEqual([]);
  });

  it("testo della scorciatoia", () => {
    expect(shortcutText({ key: "b", mod: true, shift: true })).toBe("Ctrl+Maiusc+B");
    expect(shortcutText({ key: "Escape" })).toBe("Escape");
  });
});
