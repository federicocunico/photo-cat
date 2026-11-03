import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../i18n";
import { ipcMockModule } from "../test/ipcMock";
import { checkedCleanup } from "../test/a11y";
import { emptyFilter, idsSelection, useLibrary } from "../store/library";
import { ModuleNav } from "../components/shell/ModuleNav";
import { Panels } from "../components/shell/Panels";
import { ShortcutSheet } from "../components/command/ShortcutSheet";
import { SelectionBar } from "../modules/library/SelectionBar";
import { useUndoHotkey } from "../store/undo";
import { useTagHotkeys } from "../modules/tags/useTagHotkeys";
import { entries, useShortcutCatalog } from "./catalog";
import { usePaletteCommand } from "./paletteCommand";
import { shortcutText, useCommandStore } from "./registry";
import { useSheetCommand } from "./sheetCommand";

vi.mock("../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());
beforeEach(() => {
  useCommandStore.setState({ commands: [] });
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1]) });
});

/** Tutti i gestori di scorciatoie dell'app, montati insieme. */
function App() {
  useShortcutCatalog();
  usePaletteCommand();
  useSheetCommand();
  useUndoHotkey();
  useTagHotkeys(() => ({ target: { kind: "ids", ids: [1] }, count: 1 }));
  return (
    <>
      <ModuleNav />
      <Panels left={<div />} center={<SelectionBar />} right={<div />} />
      <ShortcutSheet />
    </>
  );
}

function renderApp() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <App />
    </QueryClientProvider>,
  );
}

describe("foglio delle scorciatoie", () => {
  it("? lo apre con Generale, Libreria, Visualizzatore, Sequenza e Persone", async () => {
    renderApp();
    fireEvent.keyDown(window, { key: "?", shiftKey: true });
    const sheet = await screen.findByRole("dialog", { name: "Scorciatoie da tastiera" });
    for (const area of ["Generale", "Libreria", "Visualizzatore", "Sequenza", "Persone"])
      expect(within(sheet).getByRole("heading", { name: area })).toBeInTheDocument();
    expect(within(sheet).getByText("Ctrl+K")).toBeInTheDocument();
  });

  it("ogni scorciatoia registrata da un comando è nel foglio", () => {
    renderApp();
    const keys = entries(i18n.t).map((e) => e.keys);
    const covered = (text: string) =>
      keys.some((k) => k.split(/, | /).includes(text) || k === text) ||
      // Ctrl+1…7 e 1–9: un'unica voce per il gruppo di tasti.
      (/^Ctrl\+[1-7]$/.test(text) && keys.includes("Ctrl+1…7")) ||
      (/^[1-9]$/.test(text) && keys.some((k) => k.includes("1–9"))) ||
      (text === "0" && keys.some((k) => k === "0" || k.includes(", 0")));
    const missing = useCommandStore
      .getState()
      .commands.filter((c) => c.shortcut)
      .map((c) => shortcutText(c.shortcut!).replace("Escape", "Esc"))
      .filter((text) => !covered(text));
    expect(missing).toEqual([]);
  });
});
