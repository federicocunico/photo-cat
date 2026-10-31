import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, emit, ipcMockModule, job, resources } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLive, useLiveEvents } from "../../store/live";
import { JobsPage } from "./JobsPage";

vi.mock("../../lib/ipc", () => ipcMockModule());

function Harness() {
  useLiveEvents();
  return (
    <QueryClientProvider client={new QueryClient()}>
      <JobsPage />
    </QueryClientProvider>
  );
}

const phase = (name: string, over: object) => ({
  name,
  state: "running" as const,
  done: 0,
  failed: 0,
  total: 0,
  totalKnown: true,
  percent: 0,
  rate: null,
  etaSecs: null,
  ...over,
});

beforeEach(() => {
  useLive.setState({ jobs: {}, resources: null, errorCount: 0, tail: [] });
  vi.clearAllMocks();
});
afterEach(cleanup);

describe("schermata Job", () => {
  it("spiega nel title il totale provvisorio durante la scansione", () => {
    render(<Harness />);
    act(() =>
      emit(
        "job",
        job({
          id: 8,
          phases: [
            phase("scansione", { state: "running", done: 1204, totalKnown: false, percent: null }),
            phase("metadati", { done: 1171, total: 1204, totalKnown: false, percent: null }),
          ],
        }),
      ),
    );
    const meta = screen.getByTestId("phase-metadati");
    const count = within(meta).getByText("1171 / 1204…");
    expect(count).toHaveAttribute(
      "title",
      "1171 elaborati su 1204 trovati finora: il totale cresce finché la scansione non è terminata",
    );
    expect(
      within(screen.getByTestId("phase-scansione")).getByText("1204 trovati"),
    ).not.toHaveAttribute("title");
  });

  it("mostra fasi, percentuali, conteggi, ETA e controlli", () => {
    render(<Harness />);
    expect(screen.getByText("Nessun job. Avvia un import per iniziare.")).toBeInTheDocument();
    act(() =>
      emit(
        "job",
        job({
          id: 7,
          overallPercent: 71.3,
          etaSecs: 2280,
          failed: 4,
          phases: [
            phase("scansione", { state: "running", done: 412, totalKnown: false, percent: null }),
            phase("analisi_ai", {
              done: 6000,
              failed: 200,
              total: 10000,
              percent: 62,
              rate: 11.2,
              etaSecs: 135,
            }),
            phase("miniature", { state: "done", done: 1000, total: 1000, percent: 100 }),
          ],
        }),
      ),
    );
    const card = screen.getByTestId("job-7");
    // Scansione ancora in corso: totale complessivo non significativo.
    expect(within(card).getByTestId("overall-7")).toHaveTextContent("—");
    expect(within(card).getByText("Tempo stimato 38 min")).toBeInTheDocument();

    const scan = screen.getByTestId("phase-scansione");
    expect(scan).toHaveTextContent("412 trovati");
    expect(within(scan).getByRole("progressbar")).toHaveClass("indeterminate");

    const ai = screen.getByTestId("phase-analisi_ai");
    expect(ai).toHaveTextContent("Analisi AI");
    expect(ai).toHaveTextContent("62%");
    expect(ai).toHaveTextContent("6200 / 10.000");
    expect(ai).toHaveTextContent("11,2/s");
    expect(ai).toHaveTextContent("2 min 20 s");

    fireEvent.click(within(card).getByRole("button", { name: "Pausa" }));
    expect(commandsMock.pauseJob).toHaveBeenCalledWith(7);
    fireEvent.click(within(card).getByRole("button", { name: "Annulla" }));
    expect(commandsMock.cancelJob).toHaveBeenCalledWith(7);

    fireEvent.click(within(card).getByRole("button", { name: /4 errori · Vedi errori/ }));
    expect(useLayout.getState().module).toBe("diagnostics");
    expect(useLayout.getState().diagFilter).toEqual({ jobId: 7 });
  });

  it("a scansione conclusa mostra la percentuale complessiva", () => {
    render(<Harness />);
    act(() =>
      emit(
        "job",
        job({
          id: 8,
          overallPercent: 71.3,
          phases: [
            phase("scansione", { state: "done", done: 50, total: 50, percent: 100 }),
            phase("metadati", { done: 30, total: 50, percent: 60 }),
          ],
        }),
      ),
    );
    expect(screen.getByTestId("overall-8")).toHaveTextContent("71%");
    expect(screen.getByTestId("phase-metadati")).toHaveTextContent("30 / 50");
  });

  it("job in pausa offre Riprendi; job concluso non ha controlli", () => {
    render(<Harness />);
    act(() => emit("job", job({ id: 1, state: "paused" })));
    fireEvent.click(screen.getByRole("button", { name: "Riprendi" }));
    expect(commandsMock.resumeJob).toHaveBeenCalledWith(1);
    act(() => emit("job", job({ id: 1, state: "completed", overallPercent: 100 })));
    expect(
      within(screen.getByTestId("job-1")).queryByRole("button", { name: "Annulla" }),
    ).toBeNull();
    expect(screen.getByText("Completato")).toBeInTheDocument();
  });

  it("pannello risorse con budget e modelli residenti", () => {
    render(<Harness />);
    act(() => emit("resources", resources()));
    const panel = screen.getByRole("region", { name: "Risorse" });
    expect(panel).toHaveTextContent("Budget app");
    expect(panel).toHaveTextContent("4,0 GB");
    expect(panel).toHaveTextContent("clip");
    expect(panel).toHaveTextContent("600 MB");
    expect(panel).toHaveTextContent("8 / 4");
  });
});
