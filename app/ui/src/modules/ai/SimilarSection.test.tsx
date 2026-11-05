import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule, job } from "../../test/ipcMock";
import type { SimilarStats } from "../../bindings";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { SimilarSection } from "./SimilarSection";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const stats = (pending: number): SimilarStats => ({
  groups: 12,
  groupedPhotos: 40,
  pending,
  sizeBuckets: [
    { bucket: "two", label: "2", groups: 6, photos: 12 },
    { bucket: "three_five", label: "3-5", groups: 4, photos: 16 },
    { bucket: "over10", label: ">10", groups: 2, photos: 25 },
  ],
});

const cm = commandsMock as unknown as Record<string, ReturnType<typeof vi.fn>>;

function renderIt() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <SimilarSection />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  cm.aiSimilarStats = vi.fn(async () => stats(300));
  commandsMock.startSimilarAnalysis.mockClear();
  useLive.setState({ jobs: {} });
  useLayout.setState({ module: "ai" });
  useLibrary.getState().resetFilter();
  useLibrary.getState().setGroupSimilar(false);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

it("mostra i tre indicatori", async () => {
  renderIt();
  expect(await screen.findByTestId("kpi-groups")).toHaveTextContent("12");
  expect(screen.getByTestId("kpi-photos")).toHaveTextContent("40");
  expect(screen.getByTestId("kpi-pending")).toHaveTextContent("300");
});

it("il clic su una barra apre la Libreria raggruppata con la classe scelta", async () => {
  renderIt();
  fireEvent.click(await screen.findByTestId("bar-over10"));
  expect(useLayout.getState().module).toBe("library");
  expect(useLibrary.getState().filter.similarSize).toBe("over10");
  expect(useLibrary.getState().groupSimilar).toBe(true);
});

it("il collegamento apre le foto con simili", async () => {
  renderIt();
  fireEvent.click(await screen.findByRole("button", { name: "Mostra le foto con simili" }));
  expect(useLibrary.getState().filter.similarOnly).toBe(true);
  expect(useLibrary.getState().groupSimilar).toBe(true);
});

it("Avvia chiama il comando ed è disattivato con un'analisi attiva", async () => {
  renderIt();
  const btn = await screen.findByRole("button", { name: "Avvia analisi dei gruppi" });
  fireEvent.click(btn);
  await waitFor(() => expect(commandsMock.startSimilarAnalysis).toHaveBeenCalledTimes(1));
  act(() => useLive.getState().upsertJob(job({ id: 5, kind: "similar", overallPercent: 30 })));
  expect(screen.getByRole("button", { name: "Avvia analisi dei gruppi" })).toBeDisabled();
  expect(screen.getByRole("status")).toHaveTextContent("30%");
});

it("le foto in attesa diminuiscono dopo un avanzamento del job", async () => {
  renderIt();
  expect(await screen.findByTestId("kpi-pending")).toHaveTextContent("300");
  cm.aiSimilarStats.mockImplementation(async () => stats(120));
  vi.useFakeTimers({ toFake: ["setTimeout"] });
  act(() => useLive.getState().upsertJob(job({ id: 5, kind: "similar", overallPercent: 60 })));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2100);
  });
  vi.useRealTimers();
  await waitFor(() => expect(screen.getByTestId("kpi-pending")).toHaveTextContent("120"));
});

it("senza gruppi mostra lo stato vuoto", async () => {
  cm.aiSimilarStats.mockImplementation(async () => ({
    groups: 0,
    groupedPhotos: 0,
    pending: 0,
    sizeBuckets: [],
  }));
  renderIt();
  expect(await screen.findByText("Nessun gruppo di foto simili")).toBeInTheDocument();
});

it("errore di lettura con Riprova", async () => {
  cm.aiSimilarStats.mockImplementation(async () => {
    throw new Error("boom");
  });
  renderIt();
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Impossibile leggere lo stato dei gruppi.",
  );
  cm.aiSimilarStats.mockImplementation(async () => stats(5));
  fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  expect(await screen.findByTestId("kpi-pending")).toHaveTextContent("5");
});
