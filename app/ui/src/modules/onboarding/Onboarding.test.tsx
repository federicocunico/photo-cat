import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { Onboarding } from "./Onboarding";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.systemInfo.mockResolvedValue({
    system: { totalMb: 16384, logicalCores: 8 },
    budget: { budgetMb: 4096 },
    onnxRuntime: "ONNX Runtime 1.24",
  } as never);
  commandsMock.addSource.mockResolvedValue(3 as never);
  commandsMock.startImport.mockResolvedValue(11 as never);
});
afterEach(cleanup);

const renderIt = (onDone = vi.fn()) => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Onboarding onDone={onDone} />
    </QueryClientProvider>,
  );
  return onDone;
};
const next = () => fireEvent.click(screen.getByRole("button", { name: "Avanti" }));

it("percorre benvenuto, sistema, modelli e prima sorgente avviando l'import", async () => {
  const onDone = renderIt();
  expect(screen.getByText("1 / 4 · Benvenuto")).toBeInTheDocument();
  next();
  expect(await screen.findByText("16 GB RAM · 8 CPU · 4096 MB")).toBeInTheDocument();
  expect(screen.getByText("ONNX Runtime 1.24")).toBeInTheDocument();
  // La verifica del sistema non si può saltare.
  expect(screen.queryByRole("button", { name: "Salta" })).toBeNull();
  next();
  expect(screen.getByText("3 / 4 · Modelli AI")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Salta" })).toBeInTheDocument();
  next();
  const importa = screen.getByRole("button", { name: "Importa" });
  expect(importa).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Scegli una cartella"), {
    target: { value: "D:\\Foto" },
  });
  fireEvent.click(importa);
  await waitFor(() => expect(onDone).toHaveBeenCalled());
  expect(commandsMock.addSource).toHaveBeenCalledWith(
    expect.objectContaining({ path: "D:\\Foto", recursive: true }),
  );
  expect(commandsMock.startImport).toHaveBeenCalledWith(3);
  expect(commandsMock.completeOnboarding).toHaveBeenCalled();
});

it("il download dei modelli si può rimandare e l'onboarding resta completato", async () => {
  const onDone = renderIt();
  next();
  next();
  fireEvent.click(screen.getByRole("button", { name: "Salta" }));
  await waitFor(() => expect(onDone).toHaveBeenCalled());
  expect(commandsMock.verifyModels).not.toHaveBeenCalled();
  expect(commandsMock.completeOnboarding).toHaveBeenCalled();
});
