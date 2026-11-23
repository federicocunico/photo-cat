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
import { useAppMenuCommands } from "./appMenu";

vi.mock("../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());
beforeEach(() => {
  useCommandStore.setState({ commands: [] });
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1]) });
});

/** Tutti i gestori di scorciatoie dell'app, montati insieme. */
function App() {
  useShortcutCatalog();
  useAppMenuCommands();
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
      // Ctrl+1…9 e 1–9: un'unica voce per il gruppo di tasti.
      (/^Ctrl\+[1-9]$/.test(text) && keys.includes("Ctrl+1…9")) ||
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

describe("modello di selezione nel foglio (unified-selection)", () => {
  it("elenca frecce che selezionano, Ctrl+frecce, gesti del mouse e clic destro", () => {
    const t = i18n.getFixedT("it");
    const byId = new Map(entries(t).map((e) => [e.id, e]));
    expect(byId.get("doc.library.move")?.label).toBe(
      "Sposta il focus e seleziona la foto raggiunta",
    );
    expect(byId.get("doc.library.focusOnly")).toMatchObject({
      keys: "Ctrl+frecce",
      label: "Sposta solo il focus, senza cambiare la selezione",
    });
    expect(byId.get("doc.library.clicks")?.keys).toBe("Ctrl+clic, Maiusc+clic");
    expect(byId.get("doc.library.contextMenu")?.label).toMatch(/seleziona solo quella/);
    expect(byId.get("doc.people.extendGroups")).toMatchObject({
      keys: "Maiusc+↑ ↓",
      label: "Estendi la selezione dei gruppi",
    });
  });
});

describe("README", () => {
  it("4.7: ogni tasto della tabella delle scorciatoie del README è nel foglio", async () => {
    const fs = await import("node:fs");
    const path = await import("node:path");
    const { fileURLToPath } = await import("node:url");
    const readme = fs.readFileSync(
      path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../../README.md"),
      "utf8",
    );
    const start = readme.indexOf("**Scorciatoie principali**");
    const table = readme.slice(start).split(/\r?\n\r?\n/)[1] ?? "";
    const rows = table
      .split(/\r?\n/)
      .filter((l) => l.startsWith("| ") && !l.startsWith("| Tasto"))
      .map((l) => l.split("|")[1]!.trim());
    expect(rows.length).toBeGreaterThan(5);
    const keys = entries(i18n.getFixedT("it")).map((e) => e.keys);
    const missing = rows
      .flatMap((r) => r.split(", "))
      .filter((k) => !keys.some((s) => s === k || s.split(/, | /).includes(k)));
    expect(missing).toEqual([]);
  });
});
