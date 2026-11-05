import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ipcMockModule } from "../../../test/ipcMock";
import "../../../i18n";
import { commands } from "../../../lib/ipc";
import { useLayout } from "../../../store/layout";
import { emptyFilter, useLibrary } from "../../../store/library";
import { ResultsSection } from "../ResultsSection";
import { useCleanupPreset } from "./cleanupPreset";

vi.mock("../../../lib/ipc", () => ipcMockModule(true));

const c = commands as unknown as Record<string, ReturnType<typeof vi.fn>>;

const bins = (counts: number[]) =>
  counts.map((count, i) => ({ from: i / 20, to: (i + 1) / 20, count }));

beforeEach(() => {
  c.getTaxonomy.mockResolvedValue({
    version: 1,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [
          { id: "paesaggio", label: "Paesaggio", enabled: true, prompts: [] },
          { id: "ritratto", label: "Ritratto", enabled: true, prompts: [], threshold: 0.4 },
        ],
      },
    ],
  });
  c.listCatalogs.mockResolvedValue([
    { catalog: { id: 1, name: "Vacanze", kind: "manual", rules: null, createdAt: 0 }, items: 3, excluded: 0 },
    { catalog: { id: 9, name: "Cestino", kind: "manual", rules: null, createdAt: 0, system: "trash" }, items: 1, excluded: 0 },
  ]);
  c.aiMonthCoverage = vi.fn(async () => [
    { month: "2024-01", total: 10, analyzed: 8 },
    { month: null, total: 2, analyzed: 0 },
  ]);
  c.libraryFacetValues = vi.fn(async () => ({
    values: [
      { value: "paesaggio", count: 12 },
      { value: "ritratto", count: 5 },
    ],
    total: 2,
  }));
  c.aiLabelHistogram = vi.fn(async () => bins(Array.from({ length: 20 }, (_, i) => i + 1)));
  useLayout.setState({ module: "ai", aiSection: "results" });
  useLibrary.setState({ filter: emptyFilter() });
  useCleanupPreset.getState().clear();
});
afterEach(cleanup);

const renderIt = () =>
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ResultsSection />
    </QueryClientProvider>,
  );

async function ready() {
  return screen.findByTestId("bar-paesaggio");
}

it("mostra le barre in ordine decrescente e il clic apre la Libreria filtrata", async () => {
  renderIt();
  await ready();
  const ids = screen.getAllByTestId(/^bar-/).map((e) => e.getAttribute("data-testid"));
  expect(ids).toEqual(["bar-paesaggio", "bar-ritratto"]);
  fireEvent.click(screen.getByTestId("bar-ritratto"));
  expect(useLayout.getState().module).toBe("library");
  const f = useLibrary.getState().filter;
  expect(f.labels).toEqual([{ group: "subject", values: ["ritratto"] }]);
  expect(f.catalogId).toBeNull();
});

it("un intervallo offre Mostra in Libreria con labelScore e ambito", async () => {
  renderIt();
  await ready();
  fireEvent.change(screen.getByLabelText("Ambito"), { target: { value: "1" } });
  await waitFor(() => expect(c.aiLabelHistogram).toHaveBeenLastCalledWith(expect.objectContaining({ catalogId: 1 }), "subject", "paesaggio", 20));
  const hist = await screen.findByTestId("histogram");
  fireEvent.keyDown(hist, { key: "Enter" });
  fireEvent.click(await screen.findByRole("button", { name: "Mostra in Libreria" }));
  expect(useLayout.getState().module).toBe("library");
  const f = useLibrary.getState().filter;
  expect(f.catalogId).toBe(1);
  expect(f.labelScore).toEqual({ grp: "subject", value: "paesaggio", min: 0, max: 0.05 });
});

it("Pulisci… imposta la preimpostazione e apre Pulizia", async () => {
  renderIt();
  await ready();
  const hist = await screen.findByTestId("histogram");
  fireEvent.keyDown(hist, { key: "Enter" });
  fireEvent.click(await screen.findByRole("button", { name: "Pulisci…" }));
  expect(useLayout.getState().aiSection).toBe("cleanup");
  expect(useLayout.getState().module).toBe("ai");
  const { grp, value, maxScore, scopeCatalogId } = useCleanupPreset.getState();
  expect({ grp, value, maxScore, scopeCatalogId }).toEqual({
    grp: "subject",
    value: "paesaggio",
    maxScore: 0.05,
    scopeCatalogId: null,
  });
});

it("la soglia della voce è la linea dell'istogramma; i cataloghi di sistema non sono ambiti", async () => {
  renderIt();
  await ready();
  expect(await screen.findByTestId("histogram-threshold")).toBeInTheDocument();
  expect(screen.getByText("Soglia 0.25")).toBeInTheDocument();
  expect(screen.getByRole("option", { name: "Vacanze" })).toBeInTheDocument();
  expect(screen.queryByRole("option", { name: "Cestino" })).toBeNull();
});

it("senza foto analizzate mostra lo stato vuoto con il collegamento a Rianalisi", async () => {
  c.aiMonthCoverage = vi.fn(async () => [{ month: "2024-01", total: 4, analyzed: 0 }]);
  renderIt();
  expect(await screen.findByTestId("results-empty")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Vai a Rianalisi" }));
  expect(useLayout.getState().aiSection).toBe("reprocess");
});

it("cambiare ambito ricarica i dati con il catalogo", async () => {
  renderIt();
  await ready();
  fireEvent.change(screen.getByLabelText("Ambito"), { target: { value: "1" } });
  await waitFor(() =>
    expect(c.libraryFacetValues).toHaveBeenLastCalledWith(
      expect.objectContaining({ catalogId: 1 }),
      { kind: "label", group: "subject" },
      null,
      "count",
      0,
      200,
    ),
  );
  expect(c.aiMonthCoverage).toHaveBeenLastCalledWith(expect.objectContaining({ catalogId: 1 }));
});
