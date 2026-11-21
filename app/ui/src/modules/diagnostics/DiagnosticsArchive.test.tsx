import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, emit, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useLive, useLiveEvents } from "../../store/live";
import { StatusBar } from "../../components/shell/StatusBar";
import { DiagnosticsPage } from "./DiagnosticsPage";
import type { ErrorEntry, ErrorQuery } from "../../bindings";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const entry = (id: number): ErrorEntry => ({
  id,
  ts: "2026-10-30T14:31:03Z",
  level: "ERROR",
  code: "ACCESS_DENIED",
  message: "errors.access_denied",
  target: "pc_ingest::scan",
  jobId: 7,
  phase: "scansione",
  path: `D:\\Foto\\IMG_${id}.JPG`,
  detail: null,
  archivedAt: null,
});

// Backend simulato: due errori attivi; l'archiviazione li sposta tra le archiviate.
let active: ErrorEntry[];
let archived: ErrorEntry[];

function Events() {
  useLiveEvents();
  return null;
}

function renderAll() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <Events />
      <DiagnosticsPage />
      <StatusBar />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  active = [entry(1), entry(2)];
  archived = [];
  useLayout.setState({ diagFilter: {} });
  useLive.setState({ errorCount: 2 });
  commandsMock.listJobs.mockResolvedValue([]);
  commandsMock.getResources.mockResolvedValue(null as never);
  commandsMock.errorCount.mockResolvedValue(0);
  commandsMock.getConfig.mockResolvedValue({ logging: { level: "info" } });
  commandsMock.queryErrors.mockImplementation((async (q: ErrorQuery) =>
    q.archived ? [...archived] : [...active]) as never);
  commandsMock.errorGroups.mockResolvedValue([] as never);
  commandsMock.archiveErrors.mockImplementation((async () => {
    const n = active.length;
    archived = [...archived, ...active.map((e) => ({ ...e, archivedAt: "2026-10-31T09:00:00Z" }))];
    active = [];
    return { affected: n, activeCount: 0 };
  }) as never);
  commandsMock.unarchiveErrors.mockImplementation((async (ids: number[]) => {
    const back = archived.filter((e) => ids.includes(e.id));
    archived = archived.filter((e) => !ids.includes(e.id));
    active = [...active, ...back.map((e) => ({ ...e, archivedAt: null }))];
    return { affected: back.length, activeCount: active.length };
  }) as never);
});
afterEach(cleanup);

describe("errori archiviati", () => {
  it("Archivia tutti: indicatore a Nessun errore ed elenco predefinito vuoto", async () => {
    renderAll();
    expect(screen.getByTestId("error-count")).toHaveTextContent("2");
    await screen.findByRole("table", { name: "Errori registrati" });
    fireEvent.click(screen.getByRole("button", { name: "Archivia tutti" }));
    await waitFor(() => expect(commandsMock.archiveErrors).toHaveBeenCalled());
    expect(commandsMock.archiveErrors).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "query" }),
    );
    await waitFor(() => expect(screen.getByTestId("error-count")).toHaveTextContent("Nessun errore"));
    expect(await screen.findByText("2 errori archiviati")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.queryByRole("table", { name: "Errori registrati" })).not.toBeInTheDocument(),
    );
  });

  it("il filtro Archiviati mostra le voci e Ripristina chiama unarchive", async () => {
    archived = [{ ...entry(5), archivedAt: "2026-10-31T09:00:00Z" }];
    active = [];
    useLive.setState({ errorCount: 0 });
    renderAll();
    fireEvent.change(screen.getByLabelText("Stato"), { target: { value: "archived" } });
    await waitFor(() =>
      expect(commandsMock.queryErrors).toHaveBeenLastCalledWith(
        expect.objectContaining({ archived: true }),
      ),
    );
    await screen.findByTestId("detail-message");
    fireEvent.click(await screen.findByRole("button", { name: "Ripristina" }));
    await waitFor(() => expect(commandsMock.unarchiveErrors).toHaveBeenCalledWith([5]));
    await waitFor(() => expect(useLive.getState().errorCount).toBe(1));
  });

  it("un nuovo errore live dopo l'archiviazione incrementa il conteggio", async () => {
    renderAll();
    await screen.findByRole("table", { name: "Errori registrati" });
    fireEvent.click(screen.getByRole("button", { name: "Archivia tutti" }));
    await waitFor(() => expect(useLive.getState().errorCount).toBe(0));
    act(() =>
      emit("errorBatch", {
        counts: [{ jobId: 8, phase: "scansione", code: "X", level: "ERROR", count: 1 }],
        recent: [],
      }),
    );
    expect(useLive.getState().errorCount).toBe(1);
  });

  it("il dettaglio sceglie Archivia o Ripristina dalla voce, non dal filtro", async () => {
    renderAll();
    await screen.findByTestId("detail-message");
    expect(screen.queryByRole("button", { name: "Ripristina" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Archivia" }));
    await waitFor(() =>
      expect(commandsMock.archiveErrors).toHaveBeenCalledWith({ kind: "ids", ids: [1] }),
    );
  });

  it("Archivia il gruppo passa il job del gruppo", async () => {
    commandsMock.errorGroups.mockResolvedValue([
      {
        jobId: null,
        phase: "scansione",
        code: "ACCESS_DENIED",
        level: "ERROR",
        count: 2,
        files: 2,
        lastTs: "2026-10-30T14:31:03Z",
        message: "errors.access_denied",
      },
    ] as never);
    renderAll();
    fireEvent.click(await screen.findByTestId("archive-group-ACCESS_DENIED-scansione"));
    await waitFor(() =>
      expect(commandsMock.archiveErrors).toHaveBeenCalledWith(
        expect.objectContaining({ kind: "group", jobId: null, code: "ACCESS_DENIED" }),
      ),
    );
  });
});
