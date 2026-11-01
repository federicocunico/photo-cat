import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { ImportPage } from "./ImportPage";
import { splitPatterns } from "./formats";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

beforeEach(() => {
  vi.clearAllMocks();
  Object.assign(commandsMock, {
    listSources: vi.fn(async () => []),
    addSource: vi.fn(async () => 9),
    startImport: vi.fn(async () => 1),
    updateSource: vi.fn(async () => null),
    removeSource: vi.fn(async () => null),
  });
});
afterEach(cleanup);

const renderPage = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <ImportPage />
    </QueryClientProvider>,
  );

describe("Importa", () => {
  it("crea la sorgente con le opzioni scelte e avvia l'import", async () => {
    renderPage();
    const start = screen.getByRole("button", { name: "Avvia importazione" });
    expect(start).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "D:\\Foto\\2024" } });
    fireEvent.click(screen.getByLabelText("Monitora la cartella per nuovi file"));
    fireEvent.click(screen.getByLabelText("RAW (RAF, CR2, CR3, NEF, ARW, DNG…)"));
    fireEvent.click(screen.getByLabelText("PNG"));
    fireEvent.change(screen.getByLabelText("Ignora percorsi che corrispondono a"), {
      target: { value: "**/Cestino/** ; *.tmp" },
    });
    fireEvent.click(start);
    await waitFor(() => expect(commandsMock.startImport).toHaveBeenCalledWith(9, null));
    const arg = commandsMock.addSource.mock.calls[0][0] as unknown as {
      path: string;
      recursive: boolean;
      watch: boolean;
      includeFormats: string[];
      ignorePatterns: string[];
    };
    expect(arg.path).toBe("D:\\Foto\\2024");
    expect(arg.recursive).toBe(true);
    expect(arg.watch).toBe(true);
    expect(arg.includeFormats).toContain("nef");
    expect(arg.includeFormats).not.toContain("png");
    expect(arg.ignorePatterns).toEqual(["**/Cestino/**", "*.tmp"]);
    expect(useLayout.getState().module).toBe("jobs");
  });

  it("mostra l'errore del backend (cartella inesistente)", async () => {
    commandsMock.addSource.mockRejectedValueOnce({
      code: "NOT_FOUND",
      detail: "cartella non trovata: X:\\nope",
    } as never);
    renderPage();
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "X:\\nope" } });
    fireEvent.click(screen.getByRole("button", { name: "Avvia importazione" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("cartella non trovata");
    expect(commandsMock.startImport).not.toHaveBeenCalled();
  });

  it("separa i pattern da ignorare", () => {
    expect(splitPatterns(" a ; b\nc;; ")).toEqual(["a", "b", "c"]);
  });
});

const source = {
  id: 1,
  path: "/foto",
  recursive: true,
  includeFormats: [],
  ignorePatterns: [],
  watch: false,
  createdAt: 0,
};
const group = (ids: number[]) => ({ count: ids.length, ids });

describe("gestione delle importazioni", () => {
  beforeEach(() => {
    commandsMock.listSources.mockResolvedValue([{ source, items: 40, missing: 3 }] as never);
    useLibrary.setState({ filter: emptyFilter() });
    useLayout.setState({ module: "import" });
  });

  it("rimozione con dialogo di riepilogo; Annulla non rimuove", async () => {
    commandsMock.sourceRemovalSummary.mockResolvedValue({
      items: 40,
      edited: 3,
      catalogs: [{ name: "Estate", items: 7 }],
      orphanedPeople: ["Giulia", "Marco"],
      jobs: 2,
    } as never);
    renderPage();
    expect(await screen.findByRole("button", { name: "Aggiorna" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Rimuovi" }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("40 foto verranno tolte dal catalogo.")).toBeTruthy();
    expect(within(dialog).getByText(/3 foto hanno correzioni manuali/)).toBeTruthy();
    expect(within(dialog).getByText(/Estate \(7\)/)).toBeTruthy();
    expect(within(dialog).getByText(/Giulia, Marco/)).toBeTruthy();
    expect(
      within(dialog).getByText(/2 job in corso su questa sorgente verranno annullati/),
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Annulla" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(commandsMock.removeSource).not.toHaveBeenCalled();
  });

  it("riepilogo dell'ultimo aggiornamento: '5 mancanti' apre la Libreria su quei file", async () => {
    commandsMock.importUpdates.mockResolvedValue([
      {
        sourceId: 1,
        jobId: 7,
        finishedAt: 0,
        folder: null,
        new: group(Array.from({ length: 30 }, (_, i) => 100 + i)),
        modified: group([]),
        moved: group(Array.from({ length: 12 }, (_, i) => 200 + i)),
        missing: group([1, 2, 3, 4, 5]),
        restored: group([]),
        unchanged: 3,
        failed: 0,
      },
    ] as never);
    renderPage();
    const summary = await screen.findByTestId("update-summary-1");
    expect(within(summary).getByText("30 nuove")).toBeTruthy();
    expect(within(summary).getByText("12 spostate o rinominate")).toBeTruthy();
    fireEvent.click(
      within(summary).getByRole("button", { name: "Mostra in libreria: 5 mancanti" }),
    );
    expect(useLibrary.getState().filter.ids).toEqual([1, 2, 3, 4, 5]);
    expect(useLayout.getState().module).toBe("library");
  });

  it("aggiornamento senza modifiche", async () => {
    commandsMock.importUpdates.mockResolvedValue([
      {
        sourceId: 1,
        jobId: 8,
        finishedAt: 0,
        folder: null,
        new: group([]),
        modified: group([]),
        moved: group([]),
        missing: group([]),
        restored: group([]),
        unchanged: 40,
        failed: 0,
      },
    ] as never);
    renderPage();
    const summary = await screen.findByTestId("update-summary-1");
    expect(within(summary).getByText("Nessuna modifica")).toBeTruthy();
  });

  it("ricollega i mancanti: cartella proposta, nuova cartella e risultato", async () => {
    commandsMock.missingFolder.mockResolvedValue("/foto/Viaggio");
    commandsMock.relinkMissing.mockResolvedValue(42 as never);
    commandsMock.lastRelink.mockResolvedValue({
      sourceId: 1,
      jobId: 42,
      finishedAt: 0,
      oldFolder: "/foto/Viaggio",
      newFolder: "/archivio/Viaggio",
      relinked: 1,
      byHash: 1,
      stillMissing: 1,
      ambiguous: 1,
      outsideSources: true,
      sourceComplete: false,
    });
    commandsMock.addSource.mockResolvedValue(2 as never);
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Ricollega mancanti…" }));
    const dialog = await screen.findByRole("dialog", { name: "Ricollega i file mancanti" });
    expect(within(dialog).getByText("3 file mancanti in questa sorgente.")).toBeTruthy();
    const old = within(dialog).getByLabelText("Cartella in cui si trovavano") as HTMLInputElement;
    await waitFor(() => expect(old.value).toBe("/foto/Viaggio"));
    fireEvent.change(within(dialog).getByLabelText("Nuova posizione della cartella"), {
      target: { value: "/archivio/Viaggio" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Ricollega" }));
    await waitFor(() =>
      expect(commandsMock.relinkMissing).toHaveBeenCalledWith(
        1,
        "/foto/Viaggio",
        "/archivio/Viaggio",
      ),
    );
    expect(await within(dialog).findByText("Ricerca dei file in corso…")).toBeTruthy();
    useLive.getState().upsertJob({
      id: 42,
      kind: "relink",
      state: "completed",
      phases: [],
      overallPercent: 100,
      etaSecs: null,
      failed: 0,
      error: null,
    });
    expect(
      await within(dialog).findByText(
        "Ritrovati 2 file: 1 nello stesso percorso, 1 per impronta del contenuto.",
      ),
    ).toBeTruthy();
    expect(within(dialog).getByText("1 file resta mancante.")).toBeTruthy();
    expect(within(dialog).getByText(/più copie identiche/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Aggiungi come sorgente" }));
    await waitFor(() =>
      expect(commandsMock.addSource).toHaveBeenCalledWith(
        expect.objectContaining({ path: "/archivio/Viaggio", recursive: true }),
      ),
    );
    expect(await within(dialog).findByText("Sorgente aggiunta")).toBeTruthy();
  });

  it("rimuovi mancanti con conferma del numero di file", async () => {
    commandsMock.removeMissing.mockResolvedValue(3 as never);
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Rimuovi mancanti…" }));
    const dialog = await screen.findByRole("dialog", { name: "Rimuovere i file mancanti?" });
    expect(within(dialog).getByText(/3 file mancanti verranno tolti dalla libreria/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Rimuovi mancanti" }));
    await waitFor(() => expect(commandsMock.removeMissing).toHaveBeenCalledWith(1, true));
    expect(await within(dialog).findByText("3 file rimossi")).toBeTruthy();
  });
});
