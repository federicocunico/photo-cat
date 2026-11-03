import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { useToast } from "../../store/toasts";
import { ToastHost } from "../../components/feedback/ToastHost";
import { ImportPage } from "./ImportPage";
import { splitPatterns } from "./formats";
import { deferred } from "../../test/deferred";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

it("Elenco delle cartelle in caricamento: no false empty state and retry after failure", async () => {
  const pending = deferred<never[]>();
  commandsMock.listSources.mockImplementation(() => pending.promise);
  renderPage();
  expect(screen.getByRole("status", { name: "Caricamento…" })).toBeInTheDocument();
  expect(screen.queryByText("Nessuna cartella nella libreria")).not.toBeInTheDocument();
  pending.reject(new Error("Sources unavailable"));
  const retry = await screen.findByRole("button", { name: "Riprova" }, { timeout: 5000 });
  commandsMock.listSources.mockResolvedValue([]);
  fireEvent.click(retry);
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "Riprova" })).not.toBeInTheDocument(),
  );
});

vi.mock("../../lib/ipc", () => ipcMockModule(true));

beforeEach(() => {
  vi.clearAllMocks();
  useToast.setState({ notices: [] });
  useLive.setState({ jobs: {} });
  Object.assign(commandsMock, {
    listSources: vi.fn(async () => []),
    addSource: vi.fn(async () => 9),
    startImport: vi.fn(async () => 1),
    updateSource: vi.fn(async () => null),
    removeSource: vi.fn(async () => null),
  });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

const renderPage = () =>
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <ImportPage />
      <ToastHost />
    </QueryClientProvider>,
  );

/** Apre il menu di un pulsante da tastiera (Radix apre i menu con Invio). */
async function openMenu(name: string | RegExp) {
  const user = userEvent.setup();
  (await screen.findByRole("button", { name })).focus();
  await user.keyboard("{Enter}");
  return user;
}

describe("Importa", () => {
  it("crea la cartella con le opzioni scelte, avvia l'importazione e resta su Importa", async () => {
    useLayout.setState({ module: "import", jobFocus: null });
    renderPage();
    const start = screen.getByRole("button", { name: "Avvia importazione" });
    // Disabilitato con la ragione, ma raggiungibile da tastiera.
    expect(start).toHaveAttribute("aria-disabled", "true");
    expect(start).toHaveAccessibleDescription(/Scegli prima la cartella da importare/);
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "D:\\Foto\\2024" } });
    fireEvent.click(screen.getByLabelText("Monitora la cartella per nuovi file"));
    fireEvent.click(screen.getByLabelText("RAW (RAF, CR2, CR3, NEF, ARW, DNG…)"));
    fireEvent.click(screen.getByLabelText("PNG"));
    // I modelli da ignorare stanno in "Avanzate", chiuso all'inizio.
    expect(screen.queryByLabelText("Ignora percorsi che corrispondono a")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Avanzate" }));
    expect(screen.getByText(/salta tutte le cartelle chiamate Cestino/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Ignora percorsi che corrispondono a"), {
      target: { value: "**/Cestino/** ; *.tmp" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Avvia importazione" }));
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
    // Nessun salto automatico: un avviso offre di seguire l'importazione.
    expect(await screen.findByText("Importazione di D:\\Foto\\2024 avviata")).toBeInTheDocument();
    expect(useLayout.getState().module).toBe("import");
    fireEvent.click(screen.getByRole("button", { name: "Segui l'importazione" }));
    expect(useLayout.getState().module).toBe("jobs");
    expect(useLayout.getState().jobFocus).toBe(1);
  });

  it("cartella inesistente: messaggio comprensibile sotto il campo, dettaglio a richiesta", async () => {
    commandsMock.addSource.mockRejectedValueOnce({
      code: "NOT_FOUND",
      detail: "cartella non trovata: X:\\nope",
    } as never);
    renderPage();
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "X:\\nope" } });
    fireEvent.click(screen.getByRole("button", { name: "Avvia importazione" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "La cartella non esiste o non è raggiungibile. Scegline un'altra.",
    );
    expect(within(alert).queryByText(/cartella non trovata/)).toBeNull();
    fireEvent.click(within(alert).getByRole("button", { name: "Dettagli" }));
    expect(within(alert).getByText(/cartella non trovata: X:/)).toBeInTheDocument();
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
    const user = await openMenu("Azioni su /foto");
    await user.click(await screen.findByRole("menuitem", { name: "Rimuovi dalla libreria…" }));
    const dialog = await screen.findByRole("dialog", { name: "Rimuovere /foto dalla libreria?" });
    expect(await within(dialog).findByText("40 foto verranno tolte dal catalogo.")).toBeTruthy();
    expect(within(dialog).getByText(/3 foto hanno correzioni manuali/)).toBeTruthy();
    expect(within(dialog).getByText(/Estate \(7\)/)).toBeTruthy();
    expect(within(dialog).getByText(/Giulia, Marco/)).toBeTruthy();
    expect(
      within(dialog).getByText(/2 attività in corso su questa cartella verranno interrotte/),
    ).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Annulla" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(commandsMock.removeSource).not.toHaveBeenCalled();
  });

  it("Esc subito dopo l'apertura chiude il dialogo senza effetti", async () => {
    commandsMock.sourceRemovalSummary.mockResolvedValue({
      items: 40,
      edited: 0,
      catalogs: [],
      orphanedPeople: [],
      jobs: 0,
    } as never);
    renderPage();
    const user = await openMenu("Azioni su /foto");
    await user.click(await screen.findByRole("menuitem", { name: "Rimuovi dalla libreria…" }));
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(dialog).toContainElement(document.activeElement as HTMLElement));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(commandsMock.removeSource).not.toHaveBeenCalled();
  });

  it("Aggiorna avvia l'aggiornamento con un avviso, senza cambiare pagina", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Aggiorna" }));
    await waitFor(() => expect(commandsMock.startImport).toHaveBeenCalledWith(1, null));
    expect(await screen.findByText("Aggiornamento di /foto avviato")).toBeInTheDocument();
    expect(useLayout.getState().module).toBe("import");
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
    const user = await openMenu("Azioni su /foto");
    await user.click(await screen.findByRole("menuitem", { name: "Ricollega mancanti…" }));
    const dialog = await screen.findByRole("dialog", { name: "Ricollega i file mancanti" });
    expect(within(dialog).getByText("3 file mancanti in questa cartella.")).toBeTruthy();
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
    fireEvent.click(within(dialog).getByRole("button", { name: "Aggiungi alla libreria" }));
    await waitFor(() =>
      expect(commandsMock.addSource).toHaveBeenCalledWith(
        expect.objectContaining({ path: "/archivio/Viaggio", recursive: true }),
      ),
    );
    expect(await within(dialog).findByText("Cartella aggiunta alla libreria")).toBeTruthy();
  });

  it("rimuovi mancanti con conferma del numero di file; l'esito è un avviso", async () => {
    commandsMock.removeMissing.mockResolvedValue(3 as never);
    renderPage();
    const user = await openMenu("Azioni su /foto");
    await user.click(await screen.findByRole("menuitem", { name: "Rimuovi mancanti…" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Rimuovere i file mancanti?" });
    expect(within(dialog).getByText(/3 file mancanti verranno tolti dalla libreria/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Rimuovi mancanti" }));
    await waitFor(() => expect(commandsMock.removeMissing).toHaveBeenCalledWith(1, true));
    expect(await screen.findByText("3 file rimossi")).toBeTruthy();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });
});

it("senza cartelle: lo stato vuoto dice cosa fare; senza importazioni la colonna lo spiega", async () => {
  renderPage();
  expect(await screen.findByText("Nessuna cartella nella libreria")).toBeInTheDocument();
  expect(screen.getByText(/Nessuna importazione in corso/)).toBeInTheDocument();
});

it("importazione in corso: 'Segui l'importazione' apre Attività su quell'attività", async () => {
  act(() =>
    useLive.setState({
      jobs: {
        5: {
          id: 5,
          kind: "import",
          state: "running",
          phases: [{ name: "scansione", done: 120 }],
          failed: 0,
        } as never,
      },
    }),
  );
  renderPage();
  const panel = screen.getByRole("region", { name: "Importazione in corso" });
  expect(within(panel).getByText("120")).toBeInTheDocument();
  fireEvent.click(within(panel).getByRole("button", { name: "Segui l'importazione" }));
  expect(useLayout.getState().module).toBe("jobs");
  expect(useLayout.getState().jobFocus).toBe(5);
});

it("il titolo della pagina è quello della voce di navigazione", async () => {
  renderPage();
  expect(await screen.findByRole("heading", { level: 1, name: "Importa" })).toBeInTheDocument();
});
