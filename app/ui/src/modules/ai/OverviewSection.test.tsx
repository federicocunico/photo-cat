import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule, job, resources } from "../../test/ipcMock";
import type { AiOverview } from "../../bindings";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { OverviewSection } from "./OverviewSection";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const overview: AiOverview = {
  stages: [
    { stage: "classification", current: 800, stale: 100, queued: 0, never: 100, total: 1000 },
    { stage: "persons", current: 500, stale: 0, queued: 40, never: 460, total: 1000 },
    { stage: "faces", current: 500, stale: 0, queued: 0, never: 500, total: 1000 },
    { stage: "similar", current: 900, stale: 0, queued: 0, never: 100, total: 1000 },
    { stage: "phash", current: 1000, stale: 0, queued: 0, never: 0, total: 1000 },
  ],
};

const cm = commandsMock as unknown as Record<string, ReturnType<typeof vi.fn>>;

function renderIt() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <OverviewSection />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  cm.aiOverview = vi.fn(async () => overview);
  useLive.setState({ jobs: {}, resources: null });
  useLayout.setState({ module: "ai" });
  useLibrary.getState().resetFilter();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

it("mostra i conteggi per fase, anche con zero in coda", async () => {
  renderIt();
  const cls = await screen.findByTestId("stage-classification");
  expect(within(cls).getByText("Classificazione")).toBeInTheDocument();
  expect(within(cls).getByTestId("legend-current")).toHaveTextContent("Aggiornate: 800");
  expect(within(cls).getByTestId("legend-queued")).toHaveTextContent("In coda: 0");
  expect(within(screen.getByTestId("stage-persons")).getByTestId("legend-queued")).toHaveTextContent(
    "In coda: 40",
  );
  expect(screen.getByText("Impronte")).toBeInTheDocument();
});

it("il clic su un segmento apre la Libreria con il filtro della fase", async () => {
  renderIt();
  const faces = await screen.findByTestId("stage-faces");
  fireEvent.click(within(faces).getByTestId("segment-never"));
  expect(useLayout.getState().module).toBe("library");
  expect(useLibrary.getState().filter.analysis).toEqual({ stage: "faces", state: "never" });
});

it("un avanzamento di job ricarica con throttling, il completamento subito", async () => {
  renderIt();
  await screen.findByTestId("stage-faces");
  expect(cm.aiOverview).toHaveBeenCalledTimes(1);
  vi.useFakeTimers();
  const upsert = useLive.getState().upsertJob;
  act(() => upsert(job({ id: 1, kind: "analysis", overallPercent: 10 })));
  act(() => upsert(job({ id: 1, kind: "analysis", overallPercent: 20 })));
  act(() => upsert(job({ id: 1, kind: "analysis", overallPercent: 30 })));
  expect(cm.aiOverview).toHaveBeenCalledTimes(1);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2100);
  });
  expect(cm.aiOverview).toHaveBeenCalledTimes(2);
  act(() => upsert(job({ id: 1, kind: "analysis", state: "completed", overallPercent: 100 })));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(10);
  });
  expect(cm.aiOverview).toHaveBeenCalledTimes(3);
});

it("elenca i job AI attivi con la percentuale e le risorse", async () => {
  useLive.setState({
    jobs: { 7: job({ id: 7, kind: "similar", overallPercent: 42 }) },
    resources: resources(),
  });
  renderIt();
  const line = await screen.findByTestId("ai-job-7");
  expect(line).toHaveTextContent("42%");
  expect(screen.getByText("Memoria in uso dai modelli")).toBeInTheDocument();
  fireEvent.click(within(line).getByRole("button"));
  expect(useLayout.getState().module).toBe("jobs");
});

it("mostra l'errore con Riprova", async () => {
  cm.aiOverview = vi.fn().mockRejectedValueOnce(new Error("x")).mockResolvedValue(overview);
  renderIt();
  fireEvent.click(await screen.findByRole("button", { name: "Riprova" }));
  await waitFor(() => expect(screen.getByTestId("stage-persons")).toBeInTheDocument());
});
