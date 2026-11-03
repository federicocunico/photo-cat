import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "./i18n";
import { commandsMock, emit, ipcMockModule } from "./test/ipcMock";
import { useLayout } from "./store/layout";
import { MODULES } from "./modules/registry";
import { App } from "./App";

it("Primo avvio: does not render Library before startup and onboarding decisions", async () => {
  commandsMock.bootStatus.mockResolvedValue({
    phase: "migration",
    ready: false,
    error: null,
    logsPath: "D:/logs",
  });
  commandsMock.onboardingDone.mockResolvedValue(false);
  renderApp();
  expect(screen.queryByText("page:library")).not.toBeInTheDocument();
  expect(commandsMock.onboardingDone).not.toHaveBeenCalled();
  act(() => emit("boot", { phase: "ready", ready: true, error: null, logsPath: "D:/logs" }));
  expect(
    await screen.findByRole("heading", { name: "Benvenuto in photo-cat" }, { timeout: 10000 }),
  ).toBeInTheDocument();
  expect(screen.queryByText("page:library")).not.toBeInTheDocument();
});

it("Errore all'avvio: displays the error and logs with retry and another data folder", async () => {
  commandsMock.bootStatus.mockResolvedValue({
    phase: "migration",
    ready: false,
    error: "Catalog locked",
    logsPath: "D:/logs",
  });
  renderApp();
  expect(await screen.findByRole("alert")).toHaveTextContent("Catalog locked");
  expect(screen.getByText("Log: D:/logs")).toBeInTheDocument();
  expect(screen.queryByText("page:library")).not.toBeInTheDocument();
  expect(commandsMock.getConfig).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  await waitFor(() => expect(commandsMock.bootRetry).toHaveBeenCalledOnce());
  fireEvent.click(screen.getByRole("button", { name: "Scegli un’altra cartella dati" }));
  await waitFor(() => expect(commandsMock.bootChooseFolder).toHaveBeenCalledOnce());
});

vi.mock("./lib/ipc", () => ipcMockModule(true));

// Le pagine reali sono coperte dai loro test: qui interessa solo quale modulo è mostrato.
vi.mock("./modules/library/LibraryPage", () => ({ LibraryPage: () => <p>page:library</p> }));
vi.mock("./modules/catalogs/CatalogsPage", () => ({ CatalogsPage: () => <p>page:catalogs</p> }));
vi.mock("./modules/tags/TagsPage", () => ({ TagsPage: () => <p>page:tags</p> }));
vi.mock("./modules/sequence/SequencePage", () => ({
  SequencePage: () => <p>page:sequence</p>,
  SequenceStatus: () => null,
}));
vi.mock("./modules/people/PeopleModule", () => ({ PeopleModule: () => <p>page:people</p> }));
vi.mock("./modules/import/ImportPage", () => ({ ImportPage: () => <p>page:import</p> }));
vi.mock("./modules/jobs/JobsPage", () => ({ JobsPage: () => <p>page:jobs</p> }));
vi.mock("./modules/models/ModelsPage", () => ({ ModelsPage: () => <p>page:models</p> }));
vi.mock("./modules/organize/OrganizePage", () => ({ OrganizePage: () => <p>page:organize</p> }));
vi.mock("./modules/settings/SettingsPage", () => ({ SettingsPage: () => <p>page:settings</p> }));
vi.mock("./modules/diagnostics/DiagnosticsPage", () => ({
  DiagnosticsPage: () => <p>page:diagnostics</p>,
}));

function renderApp() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <App />
    </QueryClientProvider>,
  );
}

const info = (previousCrash: unknown) => ({
  version: "0.1.0",
  dataDir: "D:\\data",
  logsDir: "D:\\data\\logs",
  previousCrash,
});

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.bootStatus.mockResolvedValue({
    phase: "ready",
    ready: true,
    error: null,
    logsPath: "D:/logs",
  });
  localStorage.clear();
  useLayout.setState({ module: "library", theme: "dark", language: "it", diagFilter: {} });
  commandsMock.onboardingDone.mockResolvedValue(true);
  commandsMock.getConfig.mockResolvedValue({ ui: { theme: "dark", language: "it" } } as never);
  commandsMock.listJobs.mockResolvedValue([] as never);
  commandsMock.getResources.mockResolvedValue(null as never);
  commandsMock.libraryCount.mockResolvedValue(0 as never);
  commandsMock.appInfo.mockResolvedValue(info(null) as never);
});
afterEach(cleanup);

describe("moduli di navigazione", () => {
  it("il registro elenca i moduli richiesti", () => {
    expect(MODULES.map((m) => m.id)).toEqual([
      "library",
      "sequence",
      "catalogs",
      "people",
      "tags",
      "import",
      "organize",
      "jobs",
      "settings",
    ]);
  });

  it("la selezione dalla nav mostra il modulo corrispondente", async () => {
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "Moduli" }, { timeout: 10000 });
    const labels: Record<string, string> = {
      library: "Libreria",
      sequence: "Sequenza",
      catalogs: "Cataloghi",
      tags: "Tag",
      people: "Persone",
      import: "Importa",
      jobs: "Attività",
      organize: "Esporta",
      settings: "Impostazioni",
    };
    // Le voci dei moduli più il comando che riduce la navigazione.
    expect(within(nav).getAllByRole("button")).toHaveLength(MODULES.length + 1);
    for (const { id } of [...MODULES].reverse()) {
      const button = within(nav).getByRole("button", { name: labels[id] });
      fireEvent.click(button);
      expect(await screen.findByText(`page:${id}`)).toBeInTheDocument();
      expect(button).toHaveAttribute("aria-current", "page");
      expect(useLayout.getState().module).toBe(id);
    }
  });
});

describe("cattura dei crash", () => {
  it("senza crash precedente non mostra l'avviso", async () => {
    renderApp();
    await waitFor(() => expect(commandsMock.appInfo).toHaveBeenCalled());
    expect(screen.queryByText("Apri diagnostica")).not.toBeInTheDocument();
  });

  it("dopo un crash mostra l'avviso con collegamento alla Diagnostica", async () => {
    commandsMock.appInfo.mockResolvedValue(
      info({
        ts: "2026-10-30T14:31:03Z",
        message: "panicked at 'index out of bounds'",
        location: "crates/pc-ai/src/clip.rs:42:9",
        thread: "main",
      }) as never,
    );
    renderApp();
    const banner = await screen.findByText("La sessione precedente si è chiusa in modo anomalo.");
    expect(screen.getByText("panicked at 'index out of bounds'")).toBeInTheDocument();
    expect(screen.getByText("page:library")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Apri diagnostica" }));
    expect(await screen.findByText("page:settings")).toBeInTheDocument();
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
    });
    // L'avviso resta finché non viene chiuso esplicitamente.
    expect(banner).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));
    expect(
      screen.queryByText("La sessione precedente si è chiusa in modo anomalo."),
    ).not.toBeInTheDocument();
    expect(commandsMock.dismissCrashNotice).toHaveBeenCalled();
  });

  it("una sessione non chiusa regolarmente mostra ultima attività e memoria", async () => {
    commandsMock.appInfo.mockResolvedValue(
      info({
        ts: "2026-11-02T08:25:53Z",
        message: "",
        location: null,
        thread: null,
        kind: "unclean",
        lastActivity: "2026-11-02T08:25:53Z",
        memory: { appMb: 900, webviewMb: 2100, availableMb: 925, totalMb: 16000 },
      }) as never,
    );
    renderApp();
    expect(
      await screen.findByText(/L'app non si è chiusa regolarmente/, { exact: false }),
    ).toBeInTheDocument();
    expect(screen.getByText(/interfaccia 2100 MB, RAM libera 925 MB/)).toBeInTheDocument();
    expect(screen.getByText(/ultima attività/)).toBeInTheDocument();
  });
});

describe("recupero dell'interfaccia", () => {
  it("dopo app://ui-recovered mostra l'avviso con il motivo e il collegamento alla diagnostica", async () => {
    renderApp();
    await waitFor(() => expect(commandsMock.takeUiRecovery).toHaveBeenCalledTimes(1));
    expect(screen.queryByTestId("ui-recovered")).not.toBeInTheDocument();
    commandsMock.takeUiRecovery.mockResolvedValueOnce({
      ts: "2026-11-02T08:25:53Z",
      reason: "processo render, motivo 1, codice di uscita 0xc0000005",
      count: 1,
    } as never);
    act(() => emit("uiRecovered", undefined));
    const banner = await screen.findByTestId("ui-recovered");
    expect(banner).toHaveTextContent("L'interfaccia si è chiusa ed è stata ricaricata.");
    expect(banner).toHaveTextContent("Le attività in corso continuano");
    expect(banner).toHaveTextContent("processo render");
    fireEvent.click(within(banner).getByRole("button", { name: "Apri diagnostica" }));
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
    });
    fireEvent.click(within(banner).getByRole("button", { name: "Chiudi" }));
    expect(screen.queryByTestId("ui-recovered")).not.toBeInTheDocument();
  });
});
