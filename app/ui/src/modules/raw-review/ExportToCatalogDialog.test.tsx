import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import "../../i18n";
import type { RawExportReport, RawReviewExportPreflight } from "../../bindings";
import { commandsMock, ipcMockModule, job } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { ExportToCatalogDialog } from "./ExportToCatalogDialog";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const pre = (over: Partial<RawReviewExportPreflight> = {}): RawReviewExportPreflight => ({
  total: 12,
  byRating: [2, 1, 3, 2, 4, 2],
  noChoice: 0,
  notJpeg: 0,
  missing: 0,
  templateErrors: [],
  preview: [],
  existingFiles: 0,
  catalogConflict: null,
  import: { kind: "newSource", path: "D:/Foto/Dolomiti" },
  canStart: true,
  block: null,
  ...over,
});

const report = (over: Partial<RawExportReport> = {}): RawExportReport => ({
  jobId: 88,
  exported: 9,
  skipped: 3,
  failed: 0,
  errors: [],
  cancelled: false,
  import: null,
  catalog: { id: 7, name: "Dolomiti", created: true, photos: 12 },
  unresolved: 0,
  failure: null,
  ...over,
});

function mount(onOpenChange = vi.fn()) {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <ExportToCatalogDialog
        open
        onOpenChange={onOpenChange}
        sessionId="s1"
        sessionName="Dolomiti"
        presets={["Acros", "Velvia"]}
      />
    </QueryClientProvider>,
  );
}

const setDestination = (path = "D:/Foto/Dolomiti") =>
  fireEvent.change(screen.getByLabelText("Cartella di destinazione"), { target: { value: path } });
const startButton = () => screen.getByTestId("raw-export-start");

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLive.setState({ jobs: {} });
  useLayout.setState({ module: "raw-review" });
  commandsMock.rawReviewExportPreflight.mockImplementation(async () => pre());
  commandsMock.startRawReviewExport.mockResolvedValue(88);
  commandsMock.rawReviewExportReport.mockResolvedValue(null);
});
afterEach(cleanup);

describe("Esporta in un nuovo catalogo", () => {
  it("parte dal nome della sessione e non propone la cartella dei RAF", async () => {
    mount();
    expect(screen.getByLabelText(/Nome del catalogo/)).toHaveValue("Dolomiti");
    expect(screen.getByLabelText("Cartella di destinazione")).toHaveValue("");
    await waitFor(() => expect(commandsMock.rawReviewExportPreflight).toHaveBeenCalled());
    expect(commandsMock.rawReviewExportPreflight.mock.calls[0][0]).toMatchObject({
      sessionId: "s1",
      catalogName: "Dolomiti",
      destination: "",
      minRating: 0,
      template: null,
      presetAsTag: true,
      overwrite: false,
    });
  });

  it("mostra i conteggi per valutazione e il totale, aggiornati con la valutazione minima", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async (r) =>
      r.minRating === 4 ? pre({ total: 6 }) : pre({ total: 14 }),
    );
    mount();
    const counts = await screen.findByRole("list", { name: "Foto scelte per valutazione" });
    const rows = within(counts).getAllByRole("listitem");
    expect(rows.map((li) => li.textContent)).toEqual([
      "Senza stelle2",
      "1 stella1",
      "2 stelle3",
      "3 stelle2",
      "4 stelle4",
      "5 stelle2",
    ]);
    expect(await screen.findByTestId("raw-export-total")).toHaveTextContent(
      "Verranno esportate 14 foto",
    );
    fireEvent.change(screen.getByLabelText("Valutazione minima"), { target: { value: "4" } });
    await waitFor(() =>
      expect(screen.getByTestId("raw-export-total")).toHaveTextContent("Verranno esportate 6 foto"),
    );
    expect(commandsMock.rawReviewExportPreflight).toHaveBeenLastCalledWith(
      expect.objectContaining({ minRating: 4 }),
    );
  });

  it("indica quanti scatti sono esclusi e perché", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async () =>
      pre({ noChoice: 20, notJpeg: 3, missing: 2 }),
    );
    mount();
    const box = await screen.findByTestId("raw-export-excluded");
    expect(box).toHaveTextContent("20 senza scelta");
    expect(box).toHaveTextContent("3 non JPEG");
    expect(box).toHaveTextContent("2 file mancanti");
  });

  it("filtra per preset", async () => {
    mount();
    fireEvent.click(await screen.findByRole("checkbox", { name: "Velvia" }));
    await waitFor(() =>
      expect(commandsMock.rawReviewExportPreflight).toHaveBeenLastCalledWith(
        expect.objectContaining({ presets: ["Acros"] }),
      ),
    );
  });

  it("avvisa se la cartella verrà aggiunta o aggiornata nella libreria", async () => {
    mount();
    expect(await screen.findByTestId("raw-export-import")).toHaveTextContent(
      "La cartella «D:/Foto/Dolomiti» verrà aggiunta alla libreria.",
    );
    commandsMock.rawReviewExportPreflight.mockImplementation(async () =>
      pre({
        import: {
          kind: "updateFolder",
          sourceId: 1,
          sourcePath: "D:/Foto",
          folder: "D:/Foto/2026/Dolomiti",
        },
      }),
    );
    setDestination("D:/Foto/2026/Dolomiti");
    await waitFor(() =>
      expect(screen.getByTestId("raw-export-import")).toHaveTextContent(
        "Verrà aggiornata la cartella «D:/Foto/2026/Dolomiti» della libreria.",
      ),
    );
  });

  it("mostra l'anteprima dei nomi e il modello predefinito con il nome del catalogo", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async (r) =>
      pre({
        preview: r.template
          ? [
              { original: "DSCF0002.JPG", name: "Dolomiti_0001.jpg" },
              { original: "DSCF0001.JPG", name: "Dolomiti_0002.jpg" },
            ]
          : [],
      }),
    );
    mount();
    fireEvent.click(await screen.findByRole("radio", { name: "Rinomina i file con un modello" }));
    expect(screen.getByLabelText("Modello dei nomi")).toHaveValue("Dolomiti_{seq:04d}");
    const table = await screen.findByRole("table", { name: "Anteprima dei nuovi nomi" });
    expect(within(table).getByText("Dolomiti_0001.jpg")).toBeInTheDocument();
    expect(within(table).getByText("DSCF0002.JPG")).toBeInTheDocument();
    // Il modello segue il nome del catalogo finché non lo si scrive a mano.
    fireEvent.change(screen.getByLabelText(/Nome del catalogo/), { target: { value: "Alpi" } });
    expect(screen.getByLabelText("Modello dei nomi")).toHaveValue("Alpi_{seq:04d}");
    // Un segnaposto si inserisce con il pulsante.
    fireEvent.click(screen.getByRole("button", { name: /Inserisci \{preset\}/ }));
    expect(screen.getByLabelText("Modello dei nomi")).toHaveValue("Alpi_{seq:04d}{preset}");
  });

  it("un modello non valido mostra l'errore e impedisce l'avvio", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async (r) =>
      r.template
        ? pre({
            templateErrors: [{ kind: "unknown_token", start: 0, end: 4, text: "{xx}" }],
            canStart: false,
            block: "template_invalid",
          })
        : pre(),
    );
    mount();
    setDestination();
    fireEvent.click(await screen.findByRole("radio", { name: "Rinomina i file con un modello" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Parte del modello sconosciuta: {xx}",
    );
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(await screen.findByTestId("raw-export-block")).toHaveTextContent(
      "Il modello dei nomi non è valido: correggilo per continuare.",
    );
    fireEvent.click(startButton());
    expect(commandsMock.startRawReviewExport).not.toHaveBeenCalled();
  });

  it("blocca l'avvio con il motivo finché manca la destinazione", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async (r) =>
      r.destination ? pre() : pre({ canStart: false, block: "destination_invalid" }),
    );
    mount();
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(await screen.findByTestId("raw-export-block")).toHaveTextContent(
      "Scegli la cartella in cui salvare le foto.",
    );
    setDestination();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
  });

  it("rifiuta una destinazione dentro le cartelle della revisione", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async () =>
      pre({ import: null, canStart: false, block: "destination_in_review" }),
    );
    mount();
    setDestination("D:/RAF/Dolomiti/Scelte");
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(await screen.findByTestId("raw-export-block")).toHaveTextContent(
      "La cartella scelta fa parte della revisione",
    );
    expect(screen.queryByTestId("raw-export-import")).not.toBeInTheDocument();
  });

  it("nome già usato: chiede se aggiungere al catalogo esistente o cambiare nome, prima di partire", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async (r) =>
      pre({
        catalogConflict: { id: 3, name: "Dolomiti", canAdd: true },
        canStart: r.existingCatalog === "add",
        block: r.existingCatalog === "add" ? null : "catalog_name_taken",
      }),
    );
    mount();
    setDestination();
    expect(
      await screen.findByText("Esiste già un catalogo chiamato «Dolomiti»"),
    ).toBeInTheDocument();
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    fireEvent.click(startButton());
    expect(commandsMock.startRawReviewExport).not.toHaveBeenCalled();

    // "Cambia nome" porta il focus sul campo del nome.
    fireEvent.click(screen.getByRole("button", { name: "Cambia nome" }));
    expect(screen.getByLabelText(/Nome del catalogo/)).toHaveFocus();

    fireEvent.click(screen.getByRole("button", { name: "Aggiungi le foto al catalogo esistente" }));
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(startButton());
    await waitFor(() =>
      expect(commandsMock.startRawReviewExport).toHaveBeenCalledWith(
        expect.objectContaining({ existingCatalog: "add", catalogName: "Dolomiti" }),
      ),
    );
    expect(await screen.findByText("Esportazione avviata")).toBeInTheDocument();
    expect(localStorage.getItem("photo-cat.rawReviewExportDestination")).toBe("D:/Foto/Dolomiti");
  });

  it("segnala i file già presenti nella destinazione", async () => {
    commandsMock.rawReviewExportPreflight.mockImplementation(async () => pre({ existingFiles: 4 }));
    mount();
    expect(await screen.findByTestId("raw-export-existing")).toHaveTextContent(
      "4 file sono già nella cartella e verranno saltati.",
    );
    fireEvent.click(screen.getByRole("checkbox", { name: /Sovrascrivi i file già presenti/ }));
    await waitFor(() =>
      expect(screen.getByTestId("raw-export-existing")).toHaveTextContent(
        "4 file sono già nella cartella e verranno sostituiti.",
      ),
    );
  });

  it("l'opzione del tag del preset è attiva e si può togliere", async () => {
    mount();
    const box = await screen.findByRole("checkbox", { name: /Aggiungi il preset come tag/ });
    expect(box).toBeChecked();
    fireEvent.click(box);
    await waitFor(() =>
      expect(commandsMock.rawReviewExportPreflight).toHaveBeenLastCalledWith(
        expect.objectContaining({ presetAsTag: false }),
      ),
    );
  });
});

describe("Avanzamento e riepilogo", () => {
  async function started() {
    const view = mount();
    setDestination();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(startButton());
    await screen.findByText("Esportazione avviata");
    return view;
  }
  const finish = (
    state: "completed" | "cancelled" | "completed_with_errors" | "failed" = "completed",
    error: string | null = null,
  ) =>
    act(() =>
      useLive.setState({
        jobs: { 88: job({ id: 88, kind: "raw_review_export", state, error }) },
      }),
    );

  it("durante l'esportazione rimanda alle attività", async () => {
    await started();
    fireEvent.click(screen.getByRole("button", { name: "Apri attività" }));
    expect(useLayout.getState().module).toBe("jobs");
    expect(useLayout.getState().jobFocus).toBe(88);
  });

  it("a lavoro finito mostra il riepilogo e apre il catalogo", async () => {
    commandsMock.rawReviewExportReport.mockResolvedValue(report());
    await started();
    expect(commandsMock.rawReviewExportReport).not.toHaveBeenCalled();
    finish();
    const box = await screen.findByTestId("raw-export-report");
    expect(box).toHaveTextContent("Esportazione completata");
    expect(box).toHaveTextContent("9 foto esportate");
    expect(box).toHaveTextContent("3 foto saltate perché già presenti");
    expect(box).toHaveTextContent("Catalogo «Dolomiti» creato con 12 foto");
    fireEvent.click(screen.getByRole("button", { name: "Apri il catalogo" }));
    expect(useLayout.getState().module).toBe("library");
    expect(useLibrary.getState().filter.catalogId).toBe(7);
  });

  it("elenca gli errori con il nome del file e il motivo tradotto", async () => {
    commandsMock.rawReviewExportReport.mockResolvedValue(
      report({
        exported: 11,
        failed: 2,
        errors: [
          {
            name: "DSCF0003.JPG",
            sourcePath: "D:/RAF/DSCF0003.JPG",
            code: "NOT_FOUND",
            messageKey: "errors.not_found",
            reason: "No such file",
          },
          {
            name: "DSCF0004.JPG",
            sourcePath: "D:/RAF/DSCF0004.JPG",
            code: "STRANA",
            messageKey: "x",
            reason: "Motivo grezzo",
          },
        ],
      }),
    );
    await started();
    finish("completed_with_errors");
    const list = await screen.findByRole("list", { name: "Foto con errori" });
    expect(list).toHaveTextContent("DSCF0003.JPG — File o cartella non trovati.");
    expect(list).toHaveTextContent("DSCF0004.JPG — Motivo grezzo");
    expect(screen.getByText("Esportazione completata con errori")).toBeInTheDocument();
  });

  it("mostra l'annullamento: file già copiati e catalogo non creato", async () => {
    commandsMock.rawReviewExportReport.mockResolvedValue(
      report({ cancelled: true, catalog: null, exported: 5, skipped: 0 }),
    );
    await started();
    finish("cancelled");
    expect(await screen.findByText("Esportazione annullata")).toBeInTheDocument();
    expect(screen.getByText(/il catalogo non è stato creato/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Apri il catalogo" })).not.toBeInTheDocument();
  });

  it("file non trovati in libreria: riepilogo con errori anche senza errori di copia", async () => {
    commandsMock.rawReviewExportReport.mockResolvedValue(
      report({ unresolved: 2, catalog: { id: 7, name: "Dolomiti", created: true, photos: 10 } }),
    );
    await started();
    finish("completed_with_errors");
    expect(await screen.findByText("Esportazione completata con errori")).toBeInTheDocument();
    expect(
      screen.getByText("2 file non sono stati trovati in libreria dopo l'aggiornamento."),
    ).toBeInTheDocument();
  });

  it("un errore che ferma l'attività arriva nel riepilogo con il motivo", async () => {
    commandsMock.rawReviewExportReport.mockResolvedValue(
      report({
        exported: 4,
        skipped: 0,
        catalog: null,
        failure: { code: "STRANA", messageKey: "x", reason: "cartella non scrivibile" },
      }),
    );
    await started();
    finish("failed", "cartella non scrivibile");
    expect(await screen.findByText("Esportazione non completata")).toBeInTheDocument();
    expect(screen.getByText("cartella non scrivibile")).toBeInTheDocument();
    expect(screen.getByText(/il catalogo non è stato creato/)).toBeInTheDocument();
  });

  it("senza riepilogo (perso dall'app) non resta in attesa e lo spiega", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      await started();
      finish("completed");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(5000);
      });
      expect(await screen.findByText("Riepilogo non disponibile")).toBeInTheDocument();
      expect(screen.getByText(/il riepilogo non è più disponibile/)).toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: "Chiudi" }).length).toBeGreaterThan(0);
    } finally {
      vi.useRealTimers();
    }
  });
});
