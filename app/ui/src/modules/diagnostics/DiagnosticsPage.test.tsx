import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { DiagnosticsPage } from "./DiagnosticsPage";
import type { ErrorEntry } from "../../bindings";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const entry = (over: Partial<ErrorEntry>): ErrorEntry => ({
  id: 1,
  ts: "2026-10-30T14:31:03Z",
  level: "ERROR",
  code: "ACCESS_DENIED",
  message: "errors.access_denied",
  target: "pc_ingest::scan",
  jobId: 7,
  phase: "scansione",
  path: "D:\\Foto\\2023\\Privato",
  detail: "Access is denied. (os error 5)",
  ...over,
});

function renderPage() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <DiagnosticsPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.getConfig.mockResolvedValue({ logging: { level: "info" } });
  commandsMock.queryErrors.mockResolvedValue([
    entry({}),
    entry({
      id: 2,
      level: "WARN",
      code: "CORRUPT_EXIF",
      message: "errors.corrupt_exif",
      phase: "metadati",
      path: "D:\\Foto\\IMG_6620.JPG",
    }),
  ] as never);
  useLayout.setState({ diagFilter: { jobId: 7 } });
});
afterEach(cleanup);

describe("schermata Diagnostica", () => {
  it("mostra messaggi localizzati, suggerimento e dettaglio tecnico", async () => {
    renderPage();
    await screen.findByTestId("error-row-1");
    expect(screen.getByTestId("error-row-1")).toHaveTextContent(
      "Accesso negato al file o alla cartella.",
    );
    expect(screen.getByTestId("detail-message")).toHaveTextContent(
      "Accesso negato al file o alla cartella.",
    );
    expect(screen.getByTestId("detail-hint")).toHaveTextContent("Verifica i permessi");
    expect(screen.getByText(/os error 5/)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("error-row-2"));
    expect(screen.getByTestId("detail-message")).toHaveTextContent("Metadati EXIF danneggiati.");
  });

  it("apre con il filtro del job e applica livello, fase e testo", async () => {
    renderPage();
    await screen.findByTestId("error-row-1");
    expect(commandsMock.queryErrors).toHaveBeenLastCalledWith(
      expect.objectContaining({ jobId: 7, level: null }),
    );

    fireEvent.change(screen.getByLabelText("Livello"), { target: { value: "WARN" } });
    await waitFor(() =>
      expect(commandsMock.queryErrors).toHaveBeenLastCalledWith(
        expect.objectContaining({ level: "WARN", jobId: 7 }),
      ),
    );
    fireEvent.change(screen.getByLabelText("Fase"), { target: { value: "metadati" } });
    fireEvent.change(screen.getByLabelText("Testo"), { target: { value: " IMG_6620 " } });
    await waitFor(() =>
      expect(commandsMock.queryErrors).toHaveBeenLastCalledWith(
        expect.objectContaining({ level: "WARN", phase: "metadati", text: "IMG_6620" }),
      ),
    );
    fireEvent.click(screen.getByRole("button", { name: "Azzera filtri" }));
    await waitFor(() =>
      expect(commandsMock.queryErrors).toHaveBeenLastCalledWith(
        expect.objectContaining({ level: null, phase: null, jobId: null, text: null }),
      ),
    );
  });

  it("il livello di log si cambia dalla schermata", async () => {
    renderPage();
    await waitFor(() => expect(screen.getByLabelText("Livello di log")).toHaveValue("info"));
    fireEvent.change(screen.getByLabelText("Livello di log"), { target: { value: "debug" } });
    expect(commandsMock.setLogLevel).toHaveBeenCalledWith("debug");
  });

  it("riprova i file falliti del job filtrato e apre la cartella log", async () => {
    commandsMock.retryFailed.mockResolvedValue([12, 13] as never);
    renderPage();
    await screen.findByTestId("error-row-1");
    fireEvent.click(screen.getByRole("button", { name: "Riprova file falliti" }));
    expect(await screen.findByRole("status")).toHaveTextContent("accodati: 2");
    expect(commandsMock.retryFailed).toHaveBeenCalledWith(7, null);
    fireEvent.click(screen.getByRole("button", { name: "Apri cartella log" }));
    await waitFor(() => expect(commandsMock.openLogs).toHaveBeenCalled());
  });
});
