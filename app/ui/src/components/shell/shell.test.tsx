import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { emit, ipcMockModule, job, resources } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLive, useLiveEvents } from "../../store/live";
import { Panels } from "./Panels";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";

vi.mock("../../lib/ipc", () => ipcMockModule());

function Live({ children }: { children: React.ReactNode }) {
  useLiveEvents();
  return <>{children}</>;
}

beforeEach(() => {
  localStorage.clear();
  useLayout.setState({ leftOpen: true, rightOpen: true, module: "library", leftWidth: 240 });
  useLive.setState({ jobs: {}, resources: null, errorCount: 0, tail: [] });
});
afterEach(cleanup);

describe("layout a pannelli", () => {
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
      </Live>,
    );
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Nessun job attivo");
    act(() =>
      emit(
        "job",
        job({
          id: 3,
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
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Analisi AI 62%");

    act(() => emit("resources", resources()));
    expect(screen.getByTestId("ram-chip")).toHaveTextContent("RAM 5,1/16 GB");

    // Un clic porta alla schermata Job.
    fireEvent.click(screen.getByTestId("job-pill"));
    expect(useLayout.getState().module).toBe("jobs");

    act(() => emit("job", job({ id: 3, state: "completed", overallPercent: 100 })));
    expect(screen.getByTestId("job-pill")).toHaveTextContent("Nessun job attivo");
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
    act(() => {
      emit("error", { ...base, level: "ERROR", message: "a" });
      emit("error", { ...base, level: "WARN", message: "b" });
      emit("error", { ...base, level: "ERROR", message: "c" });
    });
    expect(screen.getByTestId("error-count")).toHaveTextContent("2 errori nel log");
    fireEvent.click(screen.getByTestId("error-count"));
    expect(useLayout.getState().module).toBe("diagnostics");
    expect(useLayout.getState().diagFilter).toEqual({ level: "ERROR" });
  });
});
