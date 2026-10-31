import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "./i18n";
import { commandsMock, ipcMockModule } from "./test/ipcMock";
import { useLayout } from "./store/layout";
import { MODULES } from "./modules/registry";
import { App } from "./App";

vi.mock("./lib/ipc", () => ipcMockModule(true));

// Le pagine reali sono coperte dai loro test: qui interessa solo quale modulo è mostrato.
vi.mock("./modules/library/LibraryPage", () => ({ LibraryPage: () => <p>page:library</p> }));
vi.mock("./modules/catalogs/CatalogsPage", () => ({ CatalogsPage: () => <p>page:catalogs</p> }));
vi.mock("./modules/people/PeoplePage", () => ({ PeoplePage: () => <p>page:people</p> }));
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
  it("il registro elenca i 9 moduli richiesti", () => {
    expect(MODULES.map((m) => m.id)).toEqual([
      "library",
      "catalogs",
      "people",
      "import",
      "jobs",
      "models",
      "diagnostics",
      "organize",
      "settings",
    ]);
  });

  it("la selezione dalla nav mostra il modulo corrispondente", async () => {
    renderApp();
    const nav = screen.getByRole("navigation", { name: "Moduli" });
    const labels: Record<string, string> = {
      library: "Libreria",
      catalogs: "Cataloghi",
      people: "Persone",
      import: "Importa",
      jobs: "Job",
      organize: "Esporta e organizza",
      models: "Modelli AI",
      diagnostics: "Diagnostica",
      settings: "Impostazioni",
    };
    expect(within(nav).getAllByRole("button")).toHaveLength(9);
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
    expect(await screen.findByText("page:diagnostics")).toBeInTheDocument();
    expect(useLayout.getState().module).toBe("diagnostics");
    // L'avviso resta finché non viene chiuso esplicitamente.
    expect(banner).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Chiudi" }));
    expect(
      screen.queryByText("La sessione precedente si è chiusa in modo anomalo."),
    ).not.toBeInTheDocument();
    expect(commandsMock.dismissCrashNotice).toHaveBeenCalled();
  });
});
