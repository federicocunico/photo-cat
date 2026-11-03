import { act, fireEvent, render as rtlRender, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { emit, ipcMockModule, job, resources } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLive, useLiveEvents } from "../../store/live";
import { Panels } from "./Panels";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";
import { ModuleNav } from "./ModuleNav";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { trackCommand, usePending } from "../../store/pending";
import { commandsMock } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

it("Richiesta lenta: shows feedback after 300ms and clears it when IPC completes", async () => {
  vi.useFakeTimers();
  let finish!: () => void;
  commandsMock.listSources.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = () => resolve([]);
      }),
  );
  render(<TopBar />);
  let request!: Promise<unknown>;
  act(() => {
    request = trackCommand("listSources", () => commandsMock.listSources());
  });
  act(() => {
    vi.advanceTimersByTime(299);
  });
  expect(screen.queryByTestId("pending-ipc")).not.toBeInTheDocument();
  act(() => {
    vi.advanceTimersByTime(1);
  });
  expect(screen.getByTestId("pending-ipc")).toHaveAttribute("title", "1 richieste in corso");
  await act(async () => {
    finish();
    await request;
  });
  expect(screen.queryByTestId("pending-ipc")).not.toBeInTheDocument();
  expect(usePending.getState().count).toBe(0);
  vi.useRealTimers();
});

it("Nessun job attivo: idle job pill contains no animated spinner", () => {
  render(<TopBar />);
  expect(screen.getByTestId("job-pill").querySelector(".feedback-spinner")).toBeNull();
});

vi.mock("../../lib/ipc", () => ipcMockModule());

/** Barra superiore e di stato leggono anche dalle query (menu Aiuto, modelli). */
const render = (ui: ReactElement) =>
  rtlRender(<QueryClientProvider client={new QueryClient()}>{ui}</QueryClientProvider>);

function Live({ children }: { children: React.ReactNode }) {
  useLiveEvents();
  return <>{children}</>;
}

beforeEach(() => {
  localStorage.clear();
  useLayout.setState({ leftOpen: true, rightOpen: true, module: "library", leftWidth: 240 });
  useLive.setState({ jobs: {}, resources: null, errorCount: 0, tail: [] });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

describe("layout a pannelli", () => {
  it("Ctrl+I mostra e nasconde l'ispettore, ma non mentre si scrive", () => {
    render(
      <Panels left={<div>sx</div>} center={<input aria-label="campo" />} right={<div>dx</div>} />,
    );
    fireEvent.keyDown(window, { key: "i", ctrlKey: true });
    expect(screen.queryByTestId("right-panel")).not.toBeInTheDocument();
    fireEvent.keyDown(screen.getByLabelText("campo"), { key: "i", ctrlKey: true });
    expect(screen.queryByTestId("right-panel")).not.toBeInTheDocument();
    fireEvent.keyDown(window, { key: "i", ctrlKey: true });
    expect(screen.getByTestId("right-panel")).toBeInTheDocument();
  });

  it("lo stato dei pannelli è persistito e ripristinato", async () => {
    const { unmount } = render(
      <Panels left={<div>sx</div>} center={<div>centro</div>} right={<div>dx</div>} />,
    );
    expect(screen.getByTestId("right-panel")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Comprimi ispettore" }));
    expect(screen.queryByTestId("right-panel")).not.toBeInTheDocument();
    unmount();

    // Simula un riavvio: lo stato in memoria viene perso e riletto da localStorage.
    const raw = localStorage.getItem("photo-cat.layout")!;
    expect(JSON.parse(raw).state.rightOpen).toBe(false);
    useLayout.setState({ rightOpen: true }); // stato in memoria "perso"
    localStorage.setItem("photo-cat.layout", raw); // il disco conserva il valore salvato
    await act(async () => {
      await useLayout.persist.rehydrate();
    });
    render(<Panels left={<div>sx</div>} center={<div>centro</div>} right={<div>dx</div>} />);
    expect(screen.queryByTestId("right-panel")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Espandi ispettore" })).toBeInTheDocument();
  });

  it("il pannello sinistro si ridimensiona da tastiera entro i limiti", () => {
    render(<Panels left={<div>sx</div>} center={<div />} />);
    const handle = screen.getByRole("button", { name: "Ridimensiona pannello sinistro" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(useLayout.getState().leftWidth).toBe(256);
    for (let i = 0; i < 50; i++) fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(useLayout.getState().leftWidth).toBe(420);
  });
});

describe("indicatori globali", () => {
  it("pill del job e chip RAM seguono gli eventi", () => {
    render(
      <Live>
        <TopBar />
        <StatusBar />
      </Live>,
    );
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Nessuna attività");
    act(() =>
      emit(
        "job",
        job({
          id: 3,
          kind: "analysis",
          overallPercent: 62.4,
          phases: [
            {
              name: "scansione",
              state: "done",
              done: 10,
              failed: 0,
              total: 10,
              totalKnown: true,
              percent: 100,
              rate: null,
              etaSecs: null,
            },
            {
              name: "analisi_ai",
              state: "running",
              done: 6,
              failed: 0,
              total: 10,
              totalKnown: true,
              percent: 60,
              rate: 2,
              etaSecs: 2,
            },
          ],
        }),
      ),
    );
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Analisi AI · 62%");

    act(() => emit("resources", resources()));
    expect(screen.getByTestId("ram-chip")).toHaveTextContent("RAM 5,1/16 GB");

    // Un clic porta ad Attività, su quell'attività.
    fireEvent.click(screen.getByTestId("job-pill"));
    expect(useLayout.getState()).toMatchObject({ module: "jobs", jobFocus: 3 });

    act(() => emit("job", job({ id: 3, state: "completed", overallPercent: 100 })));
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Nessuna attività");
  });

  it("la barra di stato conta gli errori e apre la diagnostica", () => {
    render(
      <Live>
        <StatusBar />
      </Live>,
    );
    expect(screen.getByTestId("error-count")).toHaveTextContent("Nessun errore");
    const base = {
      ts: new Date().toISOString(),
      code: "IO",
      target: "t",
      jobId: null,
      phase: null,
      path: null,
      detail: null,
    };
    const count = (level: string, n: number, code = "IO") => ({
      jobId: 4,
      phase: "miniature",
      code,
      level,
      count: n,
    });
    act(() => {
      emit("errorBatch", {
        counts: [count("ERROR", 1), count("WARN", 1)],
        recent: [
          { ...base, level: "ERROR", message: "a" },
          { ...base, level: "WARN", message: "b" },
        ],
      });
      emit("errorBatch", {
        counts: [count("ERROR", 1)],
        recent: [{ ...base, level: "ERROR", message: "c" }],
      });
    });
    expect(screen.getByTestId("error-count")).toHaveTextContent("2 errori nel log");
    // Un lotto con migliaia di errori: un solo aggiornamento con il conteggio esatto.
    act(() => {
      emit("errorBatch", {
        counts: [count("ERROR", 1240, "DECODE_FAILED"), count("ERROR", 8, "UNSUPPORTED_FORMAT")],
        recent: [{ ...base, level: "ERROR", message: "z" }],
      });
    });
    expect(screen.getByTestId("error-count")).toHaveTextContent("1250 errori nel log");
    fireEvent.click(screen.getByTestId("error-count"));
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
    });
    expect(useLayout.getState().diagFilter).toEqual({ level: "ERROR" });
  });
});

function renderNav() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <Live>
        <ModuleNav />
      </Live>
    </QueryClientProvider>,
  );
}

function setWindowWidth(width: number) {
  window.matchMedia = ((query: string) => {
    const max = Number(/max-width:\s*(\d+)px/.exec(query)?.[1] ?? Infinity);
    return {
      matches: width <= max,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
    };
  }) as unknown as typeof window.matchMedia;
}

describe("navigazione dei moduli", () => {
  beforeEach(() => {
    setWindowWidth(1280);
    useLayout.setState({ navCompact: false });
  });

  it("Gruppi della navigazione: Sfoglia, Organizza, Importa ed esporta, poi Attività e Impostazioni", () => {
    renderNav();
    const group = (name: string) =>
      within(screen.getByRole("group", { name }))
        .getAllByRole("button")
        .map((b) => b.textContent);
    expect(group("Sfoglia")).toEqual(["Libreria", "Sequenza"]);
    expect(group("Organizza")).toEqual(["Cataloghi", "Persone", "Tag"]);
    expect(group("Importa ed esporta")).toEqual(["Importa", "Esporta"]);
    const nav = screen.getByRole("navigation", { name: "Moduli" });
    const all = within(nav).getAllByRole("button");
    expect(all[all.length - 3]).toHaveTextContent("Attività");
    expect(all[all.length - 2]).toHaveTextContent("Impostazioni");
  });

  it("Scorciatoia di modulo: Ctrl+3 apre Cataloghi, ma non mentre si scrive", () => {
    renderNav();
    fireEvent.keyDown(window, { key: "3", ctrlKey: true });
    expect(useLayout.getState().module).toBe("catalogs");

    const input = document.createElement("input");
    document.body.appendChild(input);
    fireEvent.keyDown(input, { key: "1", ctrlKey: true });
    expect(useLayout.getState().module).toBe("catalogs");
    input.remove();
  });

  it("Navigazione a icone: Ctrl+B mostra solo le icone, con il nome per il lettore di schermo", () => {
    renderNav();
    fireEvent.keyDown(window, { key: "b", ctrlKey: true });
    expect(useLayout.getState().navCompact).toBe(true);
    const nav = screen.getByRole("navigation", { name: "Moduli" });
    expect(nav).toHaveAttribute("data-compact");
    const library = within(nav).getByRole("button", { name: "Libreria" });
    expect(library).toHaveTextContent("");
    expect(library).toHaveAttribute("aria-keyshortcuts", "Control+1");
    fireEvent.click(within(nav).getByRole("button", { name: "Espandi la navigazione" }));
    expect(useLayout.getState().navCompact).toBe(false);
  });

  it("Finestra stretta: icone senza cambiare la preferenza salvata", () => {
    setWindowWidth(900);
    renderNav();
    expect(screen.getByRole("navigation", { name: "Moduli" })).toHaveAttribute("data-compact");
    expect(useLayout.getState().navCompact).toBe(false);
  });

  it("badge: persone da rivedere e attività in corso", async () => {
    commandsMock.peopleSections.mockResolvedValueOnce({
      review: 3,
      people: 0,
      ignored: 0,
      singles: 0,
      discarded: 0,
    });
    renderNav();
    act(() => emit("job", job({ id: 1, state: "running" })));
    expect(
      await screen.findByRole("button", { name: "Persone", description: "3 persone da rivedere" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Attività", description: "1 attività in corso" }),
    ).toBeInTheDocument();
  });
});

describe("barra di stato e menu Aiuto", () => {
  it("RAM sotto pressione: icona e testo, non solo colore", () => {
    render(
      <Live>
        <StatusBar />
      </Live>,
    );
    act(() => emit("resources", { ...resources(), state: "pressure" }));
    expect(screen.getByTestId("ram-chip")).toHaveTextContent("memoria sotto pressione");
    fireEvent.click(screen.getByTestId("ram-chip"));
    expect(useLayout.getState().module).toBe("jobs");
  });

  it("stato dei modelli AI con accesso a Impostazioni", () => {
    render(<StatusBar />);
    expect(screen.queryByTestId("models-chip")).not.toBeInTheDocument();
    act(() =>
      emit("models", [
        { entry: {}, state: { state: "verified" }, selfTest: null },
        { entry: {}, state: { state: "missing" }, selfTest: null },
      ] as never),
    );
    const chip = screen.getByTestId("models-chip");
    expect(chip).toHaveTextContent("Modelli AI da scaricare");
    fireEvent.click(chip);
    expect(useLayout.getState()).toMatchObject({ module: "settings", settingsSection: "models" });
  });

  it("menu Aiuto: Diagnostica e Informazioni", async () => {
    render(<TopBar />);
    const user = userEvent.setup();
    screen.getByRole("button", { name: "Aiuto" }).focus();
    await user.keyboard("{Enter}");
    await user.click(await screen.findByRole("menuitem", { name: "Informazioni su photo-cat" }));
    expect(await screen.findByRole("dialog", { name: "photo-cat" })).toBeInTheDocument();
  });
});
