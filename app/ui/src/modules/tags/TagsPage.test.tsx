import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { TagOverview } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { TagsPage } from "./TagsPage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));

const overview = (): TagOverview => ({
  tags: [
    { name: "Città", count: 201, color: "teal", sets: ["Viaggio USA 2026"] },
    { name: "Paesaggi", count: 312, color: "blue", sets: ["Viaggio USA 2026"] },
    { name: "Scartate", count: 410, color: "gray", sets: [] },
    { name: "Sunset", count: 12, color: "coral", sets: [] },
    { name: "Tramonti", count: 38, color: "yellow", sets: ["Base", "Viaggio USA 2026"] },
  ],
  sets: [
    {
      id: 1,
      name: "Base",
      exclusive: false,
      isBase: true,
      exportKeywords: false,
      tags: [{ key: 1, name: "Tramonti" }],
      tagged: 38,
    },
    {
      id: 2,
      name: "Viaggio USA 2026",
      exclusive: true,
      isBase: false,
      exportKeywords: false,
      tags: [
        { key: 1, name: "Paesaggi" },
        { key: 2, name: "Città" },
        { key: 3, name: "Tramonti" },
      ],
      tagged: 520,
    },
  ],
  activeSet: 2,
  palette: ["blue", "teal", "yellow", "coral", "purple", "gray"],
});

function renderPage() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <TagsPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.tagOverview.mockResolvedValue(overview());
});
afterEach(cleanup);

it("shows the active set with keys and counts", async () => {
  renderPage();
  expect(await screen.findByRole("heading", { name: "Viaggio USA 2026" })).toBeInTheDocument();
  const table = screen.getByRole("table", { name: "Viaggio USA 2026" });
  expect(within(table).getByText("Città")).toBeInTheDocument();
  expect(within(table).getByText("201")).toBeInTheDocument();
  expect(screen.getByText("3 tag · 520 foto taggate")).toBeInTheDocument();
  // Pulsanti a icona con etichetta accessibile.
  expect(screen.getByRole("button", { name: "Togli Città dal set" })).toBeInTheDocument();
});

it("renames a tag from its panel", async () => {
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: "Rinomina Paesaggi" }));
  const name = await screen.findByLabelText("Nome");
  await waitFor(() => expect(name).toHaveValue("Paesaggi"));
  fireEvent.change(name, { target: { value: "Panorami" } });
  fireEvent.click(screen.getByRole("button", { name: "Rinomina" }));
  await waitFor(() => expect(commandsMock.renameTag).toHaveBeenCalledWith("Paesaggi", "Panorami"));
  expect(commandsMock.tagOverview.mock.calls.length).toBeGreaterThan(1);
});

it("proposes a merge when the new name exists, and merges only after confirmation", async () => {
  commandsMock.renameTag.mockResolvedValueOnce({ outcome: "needsMerge", target: "Tramonti" });
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: /^Sunset/ }));
  const name = screen.getByLabelText("Nome");
  await waitFor(() => expect(name).toHaveValue("Sunset"));
  fireEvent.change(name, { target: { value: "tramonti" } });
  fireEvent.click(screen.getByRole("button", { name: "Rinomina" }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent('Esiste già il tag "Tramonti"');
  expect(commandsMock.mergeTag).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Conferma" }));
  await waitFor(() => expect(commandsMock.mergeTag).toHaveBeenCalledWith("Sunset", "Tramonti"));
});

it("merges into another tag with confirmation, and cancel does nothing", async () => {
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: /^Sunset/ }));
  fireEvent.click(screen.getByText("Unisci in un altro tag…"));
  fireEvent.change(screen.getByLabelText("Tag di destinazione"), {
    target: { value: "Tramonti" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Unisci" }));
  let dialog = await screen.findByRole("alertdialog");
  fireEvent.click(within(dialog).getByRole("button", { name: "Annulla" }));
  expect(screen.queryByRole("alertdialog")).toBeNull();
  expect(commandsMock.mergeTag).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Unisci" }));
  dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent('Unire "Sunset" in "Tramonti"?');
  fireEvent.click(within(dialog).getByRole("button", { name: "Conferma" }));
  await waitFor(() => expect(commandsMock.mergeTag).toHaveBeenCalledWith("Sunset", "Tramonti"));
});

it("deletes a tag after a confirmation that shows the file count", async () => {
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: /^Scartate/ }));
  fireEvent.click(screen.getByRole("button", { name: "Elimina tag (da 410 foto)…" }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent("410 foto");
  expect(commandsMock.deleteTag).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Conferma" }));
  await waitFor(() => expect(commandsMock.deleteTag).toHaveBeenCalledWith("Scartate", true));
});

it("creates a new set", async () => {
  commandsMock.saveTagSet.mockResolvedValueOnce(7);
  renderPage();
  fireEvent.click(await screen.findByText("+ Nuovo set"));
  fireEvent.change(screen.getByLabelText("Nome del nuovo set"), { target: { value: "Eventi" } });
  fireEvent.click(screen.getByRole("button", { name: "Crea" }));
  await waitFor(() => expect(commandsMock.saveTagSet).toHaveBeenCalledWith(null, "Eventi", []));
});

it("moves between sets with the arrow keys", async () => {
  renderPage();
  const active = await screen.findByRole("button", { name: /Viaggio USA 2026/, pressed: true });
  active.focus();
  fireEvent.keyDown(active, { key: "ArrowUp" });
  const base = await screen.findByRole("button", { name: /^Base/, pressed: true });
  expect(base).toHaveFocus();
  expect(screen.getByRole("heading", { name: "Base" })).toBeInTheDocument();
});

it("reports conflicts when a set becomes exclusive and opens them in Library", async () => {
  const o = overview();
  o.sets[1].exclusive = false;
  commandsMock.tagOverview.mockResolvedValue(o);
  commandsMock.setTagSetExclusive.mockResolvedValueOnce([4, 5, 6, 7, 8, 9]);
  renderPage();
  fireEvent.click(await screen.findByLabelText(/^Esclusivo/));
  expect(await screen.findByText(/6 file hanno già due o più tag/)).toBeInTheDocument();
  fireEvent.click(screen.getByText("Apri in Libreria"));
  expect(useLibrary.getState().selected).toEqual([4, 5, 6, 7, 8, 9]);
  expect(useLayout.getState().module).toBe("library");
});
