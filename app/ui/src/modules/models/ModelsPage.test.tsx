import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, emit, ipcMockModule } from "../../test/ipcMock";
import { ModelsPage } from "./ModelsPage";
import type { ModelStatus } from "../../bindings";
import { open } from "@tauri-apps/plugin-dialog";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const entry = (id: string, role: ModelStatus["entry"]["role"]): ModelStatus["entry"] => ({
  id,
  role,
  name: id.toUpperCase(),
  version: "1",
  file: `${id}.onnx`,
  sizeBytes: 20 * 1024 * 1024,
  sha256: "583fd1110a514667812fee7d684952aaf82a99b959760c8d7dca7e0ab9839299",
  urls: ["https://example.org/m.onnx"],
  memoryMb: 100,
  gpu: true,
  preprocessing: role === "classification_image" ? "clip_rgb224" : "sface_aligned_rgb112",
  postprocessing:
    role === "classification_image" ? "normalized_embedding512" : "normalized_embedding128",
  license: "MIT",
});

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.systemInfo.mockResolvedValue({
    system: {
      totalMb: 16384,
      availableMb: 9420,
      processMb: 60,
      logicalCores: 16,
      physicalCores: 8,
    },
    budget: { budgetMb: 3768, limitedBy: "profile" },
    providers: ["cpu", "direct_ml"],
    onnxRuntime: "ORT 1.24",
    executionProvider: "auto",
    resourceProfile: "balanced",
  });
  commandsMock.listModels.mockResolvedValue([
    {
      entry: entry("clip", "classification_image"),
      state: { state: "verified" },
      selfTest: { passed: true, avgMs: 42.5, message: "embedding 512 dimensioni", at: 1 },
    },
    {
      entry: entry("sface", "face_embedding"),
      state: { state: "error", code: "CHECKSUM_MISMATCH", message: "SHA-256 non corrispondente" },
      selfTest: null,
    },
  ] as never);
});
afterEach(cleanup);

const renderPage = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <ModelsPage />
    </QueryClientProvider>,
  );

describe("Modelli AI e sistema", () => {
  it("mostra sistema, stati, self-test e aggiorna il download dagli eventi", async () => {
    renderPage();
    const clip = await screen.findByTestId("model-clip");
    expect(clip).toHaveTextContent("Verificato");
    expect(clip).toHaveTextContent("Superato · 42,5 ms");
    expect(clip).toHaveTextContent("Soggetto, ambiente, interno/esterno");
    const sface = screen.getByTestId("model-sface");
    expect(sface).toHaveTextContent("Errore");
    expect(sface).toHaveTextContent("SHA-256 non corrispondente");
    expect(screen.getByTestId("models-summary")).toHaveTextContent("1 di 2 pronti");
    expect(await screen.findByText("16,0 GB")).toBeInTheDocument();
    expect(screen.getByText("8 core / 16 thread")).toBeInTheDocument();

    fireEvent.click(within(sface).getByRole("button", { name: "Scarica" }));
    expect(commandsMock.downloadModel).toHaveBeenCalledWith("sface");

    act(() =>
      emit("models", [
        {
          entry: entry("sface", "face_embedding"),
          state: {
            state: "downloading",
            done_bytes: 9 * 1024 * 1024,
            total_bytes: 20 * 1024 * 1024,
            bytes_per_second: 2 * 1024 * 1024,
          },
          selfTest: null,
        },
      ] as never),
    );
    expect(screen.getByTestId("model-sface")).toHaveTextContent("Download 45%");
    expect(screen.getByTestId("model-sface")).toHaveTextContent("2,0 MB/s");
  });

  it("verifica, test e provider", async () => {
    renderPage();
    await screen.findByTestId("model-clip");
    fireEvent.click(screen.getByRole("button", { name: "Verifica tutti" }));
    expect(commandsMock.verifyModels).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Esegui tutti i test" }));
    expect(commandsMock.selfTestModels).toHaveBeenCalledWith([]);
    fireEvent.change(screen.getByLabelText("Provider di esecuzione"), { target: { value: "cpu" } });
    expect(commandsMock.setExecutionProvider).toHaveBeenCalledWith("cpu");
    fireEvent.change(screen.getByLabelText("Profilo risorse"), { target: { value: "light" } });
    expect(commandsMock.setResourceProfile).toHaveBeenCalledWith("light");
    expect(screen.getByRole("option", { name: "CUDA (GPU NVIDIA)" })).toBeDisabled();
    expect(screen.getByRole("option", { name: "CoreML (Apple)" })).toBeDisabled();
  });

  it("mostra stati mancanti, aggiornamenti e test falliti con azioni coerenti", async () => {
    commandsMock.listModels.mockResolvedValue([
      { entry: entry("missing", "face_embedding"), state: { state: "missing" }, selfTest: null },
      {
        entry: entry("update", "face_embedding"),
        state: { state: "update_available" },
        selfTest: null,
      },
      {
        entry: entry("failed", "face_embedding"),
        state: { state: "self_test_failed", message: "Fuori tolleranza" },
        selfTest: { passed: false, avgMs: 0, message: "Fuori tolleranza", at: 1 },
      },
    ] as never);
    renderPage();
    const missing = await screen.findByTestId("model-missing");
    expect(missing).toHaveTextContent("Non presente");
    expect(within(missing).getByRole("button", { name: "Test" })).toBeDisabled();
    expect(screen.getByTestId("model-update")).toHaveTextContent("Aggiornamento disponibile");
    const failed = screen.getByTestId("model-failed");
    expect(failed).toHaveTextContent("Test fallito");
    commandsMock.selfTestModels.mockRejectedValueOnce({ detail: "Runtime non disponibile" });
    fireEvent.click(within(failed).getByRole("button", { name: "Test" }));
    expect(await within(failed).findByRole("alert")).toHaveTextContent("Runtime non disponibile");
  });

  it("installa da file e mostra gli errori dei comandi globali", async () => {
    vi.mocked(open).mockResolvedValueOnce("D:\\models\\clip.onnx");
    renderPage();
    const clip = await screen.findByTestId("model-clip");
    fireEvent.click(within(clip).getByRole("button", { name: "Installa da file…" }));
    await vi.waitFor(() =>
      expect(commandsMock.installModelFile).toHaveBeenCalledWith("clip", "D:\\models\\clip.onnx"),
    );
    commandsMock.verifyModels.mockRejectedValueOnce({ detail: "Accesso negato ai modelli" });
    fireEvent.click(screen.getByRole("button", { name: "Verifica tutti" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Accesso negato ai modelli");
  });
});
