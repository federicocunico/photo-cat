import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type {
  CameraDiagnostics,
  CameraPresets,
  CameraStatus,
  DevelopReport,
  OpenedReview,
  RawReviewDeveloped,
  VariantPlan,
} from "../../bindings";
import { commandsMock, emit, ipcMockModule, job } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
import { useRawReview } from "../../store/rawReview";
import { useToast } from "../../store/toasts";
import { CameraCheck } from "./CameraCheck";
import { DevelopDialog } from "./DevelopDialog";
import { RawReviewPage } from "./RawReviewPage";
import { useDevelopedEvents } from "./useDevelopedEvents";
import { useCameraMonitor } from "./useCamera";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const GB = 1024 ** 3;

const ready = (over: Partial<CameraStatus> = {}): CameraStatus => ({
  available: true,
  state: "ready",
  model: "X-T5",
  firmware: "4.31",
  message: null,
  simulated: false,
  ...over,
});
const diagnostics = (over: Partial<CameraDiagnostics> = {}): CameraDiagnostics => ({
  checks: [
    {
      id: "device",
      label: "Fotocamera Fujifilm vista da Windows",
      status: "ok",
      detail: null,
      hint: null,
    },
  ],
  model: "X-T5",
  firmware: "4.31",
  simulated: false,
  checkedAt: Date.UTC(2025, 0, 10, 9, 0, 0),
  ready: true,
  unresponsive: false,
  running: false,
  ...over,
});
const presetList: CameraPresets = {
  presets: [
    {
      id: "asshot",
      bank: null,
      name: "Come scattato",
      folderName: "Come scattato",
      filmSimulation: null,
      summary: "Impostazioni del RAF",
      unappliedProperties: [],
    },
    {
      id: "C1",
      bank: 1,
      name: "Kodachrome 64",
      folderName: "C1 Kodachrome 64",
      filmSimulation: "Classic Chrome",
      summary: "Classic Chrome · DR100",
      unappliedProperties: ["Color Chrome FX Blue"],
    },
    {
      id: "C2",
      bank: 2,
      name: "Acros",
      folderName: "C2 Acros",
      filmSimulation: "Acros",
      summary: "Acros · DR200",
      unappliedProperties: [],
    },
  ],
  selectedIds: ["asshot", "C1"],
};
const plan = (over: Partial<VariantPlan> = {}): VariantPlan => ({
  sourceRoot: "D:/Foto/Dolomiti",
  outputDir: "D:/Foto/Dolomiti/FujiView varianti",
  rafCount: 160,
  presets: [],
  totalConversions: 320,
  pendingConversions: 300,
  alreadyDone: 20,
  estimatedBytes: 8 * GB,
  freeBytes: 100 * GB,
  safetyMarginBytes: 2 * GB,
  enoughSpace: true,
  estimatedSeconds: 600,
  warnings: [],
  ...over,
});

function mount(selectedKeys: string[] = []) {
  const onOpenChange = vi.fn();
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <DevelopDialog open onOpenChange={onOpenChange} sessionId="s1" selectedKeys={selectedKeys} />
    </QueryClientProvider>,
  );
  return onOpenChange;
}

const startButton = () => screen.getByTestId("raw-develop-start");
const lastPlanRequest = () => {
  const calls = commandsMock.rawReviewDevelopPlan.mock.calls;
  return calls[calls.length - 1][0];
};

beforeEach(() => {
  vi.clearAllMocks();
  useLive.setState({ jobs: {} });
  useToast.setState({ notices: [] });
  commandsMock.rawReviewCameraStatus.mockResolvedValue(ready());
  commandsMock.rawReviewCameraDiagnostics.mockResolvedValue(diagnostics());
  commandsMock.rawReviewCameraPresets.mockResolvedValue(presetList);
  commandsMock.rawReviewCameraLog.mockResolvedValue([]);
  commandsMock.rawReviewDevelopPlan.mockResolvedValue(plan());
  commandsMock.startRawReviewDevelop.mockResolvedValue(90);
  commandsMock.rawReviewDevelopReport.mockResolvedValue(null);
});
afterEach(cleanup);

describe("Sviluppa dalla fotocamera", () => {
  it("con la fotocamera pronta preseleziona le ricette ricordate e permette di avviare", async () => {
    mount();
    expect(await screen.findByRole("checkbox", { name: "Come scattato" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "C1 Kodachrome 64" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "C2 Acros" })).not.toBeChecked();
    // Il riepilogo di ogni ricetta e le impostazioni non applicabili sono visibili.
    expect(screen.getByText("Classic Chrome · DR100")).toBeInTheDocument();
    expect(
      screen.getByText(/Non applicabile alla conversione: Color Chrome FX Blue/),
    ).toBeInTheDocument();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    expect(commandsMock.rawReviewDevelopPlan).toHaveBeenCalled();
    expect(lastPlanRequest()).toMatchObject({
      sessionId: "s1",
      shotKeys: null,
      presetIds: ["asshot", "C1"],
      outputDir: null,
      overwrite: false,
    });
    const summary = screen.getByTestId("raw-develop-plan");
    expect(within(summary).getByText("160")).toBeInTheDocument();
    expect(within(summary).getByText("320")).toBeInTheDocument();
    expect(within(summary).getByText(/8,0 GB necessari, 100,0 GB liberi/)).toBeInTheDocument();
    // La cartella predefinita viene dal piano.
    expect(screen.getByLabelText("Cartella delle varianti")).toHaveValue(
      "D:/Foto/Dolomiti/FujiView varianti",
    );
  });

  it("con la fotocamera non pronta disattiva Avvia e dice cosa fare", async () => {
    commandsMock.rawReviewCameraStatus.mockResolvedValue(ready({ state: "notReady" }));
    commandsMock.rawReviewCameraDiagnostics.mockResolvedValue(
      diagnostics({
        ready: false,
        checks: [
          {
            id: "usbMode",
            label: "Modalità USB: RAW CONV.",
            status: "error",
            detail: "La fotocamera è in modalità lettore di schede.",
            hint: "Nel menu della fotocamera scegli USB RAW CONV./BACKUP RESTORE.",
          },
        ],
      }),
    );
    mount();
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(startButton()).toHaveAccessibleDescription(/USB RAW CONV\./);
    expect(commandsMock.rawReviewCameraPresets).not.toHaveBeenCalled();
    expect(
      screen.getByText("Le ricette compaiono quando la fotocamera è pronta."),
    ).toBeInTheDocument();
    // Le istruzioni per la fotocamera sono aperte.
    expect(screen.getByText(/Disattiva lo spegnimento automatico/)).toBeVisible();
    expect(screen.getByText(/Chiudi FUJIFILM X RAW Studio/)).toBeVisible();
  });

  it("con la verifica non superata anche a fotocamera pronta Avvia resta spento", async () => {
    commandsMock.rawReviewCameraDiagnostics.mockResolvedValue(
      diagnostics({
        ready: false,
        checks: [
          {
            id: "autoPowerOff",
            label: "Spegnimento automatico",
            status: "error",
            detail: "attivo",
            hint: "Disattivalo.",
          },
        ],
      }),
    );
    mount();
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(startButton()).toHaveAccessibleDescription(/La verifica ha trovato un problema/);
    expect(await screen.findByText("Disattivalo.")).toBeInTheDocument();
  });

  it("una cartella di output rifiutata mostra il motivo tradotto e cosa fare", async () => {
    commandsMock.rawReviewDevelopPlan.mockRejectedValue({
      code: "OUTPUT_FOLDER_REJECTED",
      message_key: "errors.output_folder_rejected",
      detail: "La cartella di output non può essere la cartella dei RAF: D:\\foto.",
    });
    mount();
    expect(await screen.findByText("Cartella di output non ammessa.")).toBeInTheDocument();
    expect(screen.getByText(/La predefinita «FujiView varianti» va sempre bene/)).toBeInTheDocument();
    expect(screen.getByText(/non può essere la cartella dei RAF/)).toBeInTheDocument();
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
  });

  it("blocca l'avvio se lo spazio non basta e mostra spazio richiesto e libero", async () => {
    commandsMock.rawReviewDevelopPlan.mockResolvedValue(
      plan({ enoughSpace: false, estimatedBytes: 30 * GB, freeBytes: 20 * GB }),
    );
    mount();
    expect(await screen.findByText("Spazio su disco insufficiente")).toBeInTheDocument();
    expect(screen.getByText(/Servono circa 30,0 GB e ne restano 20,0 GB/)).toBeInTheDocument();
    await waitFor(() =>
      expect(startButton()).toHaveAccessibleDescription(/Spazio su disco insufficiente per/),
    );
    expect(startButton()).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(startButton());
    expect(commandsMock.startRawReviewDevelop).not.toHaveBeenCalled();
  });

  it("«Solo i selezionati (N)» invia le chiavi degli scatti", async () => {
    mount(["dscf0001", "dscf0003"]);
    const only = await screen.findByRole("radio", { name: "Solo i selezionati (2)" });
    expect(only).toBeChecked();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    expect(lastPlanRequest().shotKeys).toEqual(["dscf0001", "dscf0003"]);
    fireEvent.click(startButton());
    await waitFor(() => expect(commandsMock.startRawReviewDevelop).toHaveBeenCalled());
    expect(commandsMock.startRawReviewDevelop.mock.calls[0][0]).toMatchObject({
      sessionId: "s1",
      shotKeys: ["dscf0001", "dscf0003"],
      presetIds: ["asshot", "C1"],
    });
  });

  it("senza selezione sviluppa tutta la cartella e spiega come selezionare", async () => {
    mount([]);
    const only = await screen.findByRole("radio", { name: "Solo i selezionati (0)" });
    expect(only).toBeDisabled();
    expect(screen.getByRole("radio", { name: "Tutti gli scatti della cartella" })).toBeChecked();
    expect(screen.getByText(/selezionali prima nell'elenco della sessione/)).toBeInTheDocument();
  });

  it("scelte e opzioni finiscono nella richiesta: ricette, cartella e «Sovrascrivi»", async () => {
    mount();
    fireEvent.click(await screen.findByRole("checkbox", { name: "C2 Acros" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Sovrascrivi le varianti esistenti/ }));
    fireEvent.change(screen.getByLabelText("Cartella delle varianti"), {
      target: { value: "E:/Varianti" },
    });
    await waitFor(() =>
      expect(lastPlanRequest()).toMatchObject({
        presetIds: ["asshot", "C1", "C2"],
        outputDir: "E:/Varianti",
        overwrite: true,
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Usa la cartella predefinita" }));
    await waitFor(() => expect(lastPlanRequest().outputDir).toBeNull());
  });

  it("senza ricette scelte Avvia resta spento", async () => {
    mount();
    fireEvent.click(await screen.findByRole("checkbox", { name: "Come scattato" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "C1 Kodachrome 64" }));
    await waitFor(() => expect(startButton()).toHaveAttribute("aria-disabled", "true"));
    expect(startButton()).toHaveAccessibleDescription(/Scegli almeno una ricetta/);
  });

  it("l'avvio restituisce l'attività: avanzamento, pausa con motivo e ripresa", async () => {
    mount();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(startButton());
    expect(await screen.findByText("Sviluppo in corso")).toBeInTheDocument();
    act(() => {
      useLive.getState().upsertJob(
        job({
          id: 90,
          kind: "raw_review_develop",
          overallPercent: 42,
          phases: [{ name: "sviluppo_raw", state: "running", percent: 42 } as never],
        }),
      );
    });
    expect(screen.getByRole("progressbar", { name: "Avanzamento dello sviluppo" })).toHaveValue(42);
    expect(screen.getByText("42% completato")).toBeInTheDocument();

    commandsMock.rawReviewCameraStatus.mockResolvedValue(ready({ state: "disconnected" }));
    act(() => {
      useLive.getState().upsertJob(
        job({
          id: 90,
          kind: "raw_review_develop",
          state: "paused",
          overallPercent: 42,
          pauseReason: { code: "camera_disconnected", detail: "Ricollega il cavo USB." },
        }),
      );
      emit("rawReviewCameraStatus", ready({ state: "disconnected" }));
    });
    expect(await screen.findByText("Sviluppo in pausa")).toBeInTheDocument();
    expect(screen.getByText("Fotocamera scollegata")).toBeInTheDocument();
    expect(screen.getByText("Ricollega il cavo USB.")).toBeInTheDocument();
    // Il suggerimento tradotto del motivo della pausa.
    expect(
      screen.getByText(/Ricollega il cavo USB e accendi la fotocamera; quando risulta pronta/),
    ).toBeInTheDocument();
    // Finché la fotocamera non è pronta non si riprende.
    expect(screen.getByRole("button", { name: "Riprendi" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    act(() => emit("rawReviewCameraStatus", ready()));
    const resume = screen.getByRole("button", { name: "Riprendi" });
    await waitFor(() => expect(resume).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(resume);
    expect(commandsMock.resumeJob).toHaveBeenCalledWith(90);

    fireEvent.click(screen.getByRole("button", { name: "Apri attività" }));
    expect(useLayout.getState().module).toBe("jobs");
  });

  it("a sviluppo finito mostra il riepilogo con il RAF rifiutato e il motivo tradotto", async () => {
    const report: DevelopReport = {
      jobId: 90,
      sessionId: "s1",
      outputDir: "D:/Foto/Dolomiti/FujiView varianti",
      converted: 298,
      skipped: 20,
      failed: 2,
      failures: [
        {
          raf: "DSCF0007.RAF",
          presets: ["C1 Kodachrome 64", "C2 Acros"],
          code: "RAW_CAMERA_REJECTED",
          messageKey: "errors.raw_camera_rejected",
          reason: "rejected",
        },
      ],
      cancelled: false,
      stopped: null,
    };
    commandsMock.rawReviewDevelopReport.mockResolvedValue(report);
    mount();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(startButton());
    await screen.findByText("Sviluppo in corso");
    act(() => {
      useLive
        .getState()
        .upsertJob(job({ id: 90, kind: "raw_review_develop", state: "completed_with_errors" }));
    });
    const box = await screen.findByTestId("raw-develop-report");
    expect(within(box).getByRole("status")).toHaveTextContent("Sviluppo completato con errori");
    expect(box).toHaveTextContent("298 varianti create");
    expect(box).toHaveTextContent("20 già presenti, saltate");
    expect(box).toHaveTextContent("2 non riuscite");
    expect(box).toHaveTextContent("DSCF0007.RAF");
    expect(box).toHaveTextContent("RAF non scattato da questa fotocamera.");
    expect(box).toHaveTextContent("Ricette non sviluppate: C1 Kodachrome 64, C2 Acros");
    expect(box).toHaveTextContent("Le nuove varianti sono già tra i pannelli del confronto.");
  });

  it("se l'avvio viene rifiutato mostra il motivo e resta nel dialogo", async () => {
    commandsMock.startRawReviewDevelop.mockRejectedValue({
      code: "INVALID_INPUT",
      detail: "Uno sviluppo è già in corso",
    });
    mount();
    await waitFor(() => expect(startButton()).not.toHaveAttribute("aria-disabled"));
    fireEvent.click(startButton());
    expect(await screen.findByRole("alert")).toHaveTextContent("Uno sviluppo è già in corso");
    expect(startButton()).toBeInTheDocument();
  });
});

function CameraHarness() {
  const monitor = useCameraMonitor(false);
  return <CameraCheck monitor={monitor} />;
}

describe("Verifica della fotocamera", () => {
  it("elenca i controlli con l'esito e il suggerimento solo per quelli non superati", async () => {
    commandsMock.rawReviewCameraStatus.mockResolvedValue(ready({ state: "notReady" }));
    commandsMock.rawReviewCameraDiagnostics.mockResolvedValue(
      diagnostics({
        ready: false,
        checks: [
          {
            id: "device",
            label: "x",
            status: "ok",
            detail: "Vista come X-T5",
            hint: "non mostrare",
          },
          {
            id: "usbMode",
            label: "x",
            status: "error",
            detail: "Modalità lettore di schede",
            hint: "Scegli USB RAW CONV./BACKUP RESTORE.",
          },
        ],
      }),
    );
    render(<CameraHarness />);
    fireEvent.click(await screen.findByRole("button", { name: "Ripeti verifica" }));
    const list = await screen.findByRole("list", { name: "Controlli" });
    const items = within(list).getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Superato: Fotocamera Fujifilm vista da Windows");
    expect(items[0]).not.toHaveTextContent("non mostrare");
    expect(items[1]).toHaveTextContent("Non superato: Modalità USB: RAW CONV.");
    expect(items[1]).toHaveTextContent("Scegli USB RAW CONV./BACKUP RESTORE.");
    expect(screen.getByRole("status")).toHaveTextContent("Ci sono cose da sistemare");
  });

  it("espone il registro recente, anche con le righe in arrivo dagli eventi", async () => {
    commandsMock.rawReviewCameraLog.mockResolvedValue([
      { at: 0, level: "info", message: "Sessione aperta" },
    ]);
    render(<CameraHarness />);
    expect(await screen.findByText("Registro della fotocamera (1)")).toBeInTheDocument();
    act(() => emit("rawReviewCameraLog", { at: 1000, level: "warn", message: "Timeout" }));
    expect(await screen.findByText("Registro della fotocamera (2)")).toBeInTheDocument();
    expect(screen.getByLabelText("Registro recente della fotocamera")).toHaveTextContent("Timeout");
  });
});

const reviewed = (over: Partial<OpenedReview> = {}): OpenedReview => ({
  sessionId: "s1",
  session: {
    schemaVersion: 2,
    name: "Dolomiti",
    sourceRoot: "D:/Foto/Dolomiti",
    layout: null,
    variantsOutputDir: null,
    presets: ["Acros"],
    guidePreset: "Acros",
    panelPresets: ["Acros"],
    hiddenPresets: [],
    decisions: { dscf0002: { key: "dscf0002", rating: 4, winnerPreset: "Acros", reviewed: true } },
    lastShotKey: null,
    createdAt: "2025-01-10T09:00:00.000Z",
    updatedAt: "2025-01-10T09:00:00.000Z",
  },
  layout: "presets",
  presets: ["Acros"],
  guidePreset: "Acros",
  shots: [
    { key: "dscf0001", displayName: "DSCF0001", variants: [] },
    { key: "dscf0002", displayName: "DSCF0002", variants: [] },
  ],
  warnings: [],
  ...over,
});

function mountPage() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <RawReviewPage />
    </QueryClientProvider>,
  );
}

describe("Voce «Sviluppa dalla fotocamera» nella revisione", () => {
  beforeEach(() => {
    commandsMock.rawReviewRecent.mockResolvedValue([]);
    useRawReview.getState().clear();
    useRawReview.getState().setOpened(reviewed());
  });

  it("su un sistema senza sviluppo dalla fotocamera è disattivata con il motivo", async () => {
    commandsMock.rawReviewCameraStatus.mockResolvedValue({
      available: false,
      state: "disconnected",
      model: null,
      firmware: null,
      message: "Disponibile solo su Windows",
      simulated: false,
    });
    mountPage();
    const develop = () => screen.getByRole("button", { name: "Sviluppa dalla fotocamera…" });
    await screen.findByRole("button", { name: "Sviluppa dalla fotocamera…" });
    await waitFor(() => expect(develop()).toHaveAttribute("aria-disabled", "true"));
    const button = develop();
    expect(button).toHaveAccessibleDescription("Non disponibile: Disponibile solo su Windows");
    fireEvent.click(button);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    // Il resto del modulo funziona: esportazione e valutazione restano disponibili.
    expect(
      screen.getByRole("button", { name: "Esporta in un nuovo catalogo…" }),
    ).not.toHaveAttribute("aria-disabled");
  });

  it("su Windows apre il dialogo", async () => {
    mountPage();
    const button = await screen.findByRole("button", { name: "Sviluppa dalla fotocamera…" });
    expect(button).not.toHaveAttribute("aria-disabled");
    fireEvent.click(button);
    expect(
      await screen.findByRole("dialog", { name: "Sviluppa dalla fotocamera" }),
    ).toBeInTheDocument();
    await waitFor(() => expect(commandsMock.rawReviewCameraDiagnostics).toHaveBeenCalled());
  });
});

describe("Fine dello sviluppo", () => {
  const developed = (over: Partial<RawReviewDeveloped> = {}): RawReviewDeveloped => ({
    jobId: 90,
    sessionId: "s1",
    converted: 10,
    skipped: 2,
    failed: 0,
    cancelled: false,
    stopped: null,
    opened: reviewed({
      presets: ["Acros", "C1 Kodachrome 64"],
      session: {
        ...reviewed().session,
        presets: ["Acros", "C1 Kodachrome 64"],
        panelPresets: ["Acros", "C1 Kodachrome 64"],
      },
    }),
    refreshError: null,
    ...over,
  });

  function Host() {
    useDevelopedEvents();
    return null;
  }

  beforeEach(() => {
    useToast.setState({ notices: [] });
    useRawReview.getState().clear();
    useRawReview.getState().setOpened(reviewed());
    useRawReview.getState().setShotIndex(1);
  });

  it("aggiorna i pannelli della sessione aperta senza toccare scelte e scatto mostrato", () => {
    render(<Host />);
    act(() => emit("rawReviewDeveloped", developed()));
    const state = useRawReview.getState();
    expect(state.opened?.session.panelPresets).toEqual(["Acros", "C1 Kodachrome 64"]);
    expect(state.opened?.session.decisions?.dscf0002?.rating).toBe(4);
    expect(state.shotIndex).toBe(1);
    const [notice] = useToast.getState().notices;
    expect(notice.message).toContain("10 varianti create, 2 già presenti");
    expect(notice.message).toContain("Le nuove varianti sono nel confronto.");
  });

  it("ignora la sessione di un'altra cartella ma avvisa del risultato", () => {
    render(<Host />);
    act(() => emit("rawReviewDeveloped", developed({ sessionId: "altra" })));
    expect(useRawReview.getState().opened?.session.panelPresets).toEqual(["Acros"]);
    expect(useToast.getState().notices).toHaveLength(1);
    expect(useToast.getState().notices[0].action?.label).toBe("Apri la revisione");
  });

  it("se la rilettura non è riuscita avvisa di riaprire la cartella", () => {
    render(<Host />);
    act(() =>
      emit("rawReviewDeveloped", developed({ opened: null, refreshError: "cartella sparita" })),
    );
    const [notice] = useToast.getState().notices;
    expect(notice.tone).toBe("error");
    expect(notice.message).toContain("chiudi e riapri la cartella");
    expect(notice.message).toContain("cartella sparita");
    expect(useRawReview.getState().opened?.session.panelPresets).toEqual(["Acros"]);
  });

  it("segnala gli errori e gli annullamenti", () => {
    render(<Host />);
    act(() => emit("rawReviewDeveloped", developed({ failed: 3 })));
    expect(useToast.getState().notices[0].message).toContain("3 non riuscite");
    expect(useToast.getState().notices[0].tone).toBe("error");
    act(() => emit("rawReviewDeveloped", developed({ cancelled: true })));
    expect(useToast.getState().notices[0].message).toContain("annullato");
  });

  it("uno sviluppo fermato da un errore lo dice con il motivo tradotto e cosa fare", () => {
    render(<Host />);
    act(() =>
      emit(
        "rawReviewDeveloped",
        developed({
          converted: 3,
          stopped: {
            code: "CAMERA_UNRESPONSIVE",
            messageKey: "errors.camera_unresponsive",
            reason: "dettaglio tecnico",
          },
        }),
      ),
    );
    const [notice] = useToast.getState().notices;
    expect(notice.tone).toBe("error");
    expect(notice.message).toContain("Sviluppo fermato");
    expect(notice.message).toContain("La fotocamera non risponde.");
    expect(notice.message).toContain("Spegni e riaccendi la fotocamera");
    expect(notice.message).not.toContain("Sviluppo finito");
  });
});
