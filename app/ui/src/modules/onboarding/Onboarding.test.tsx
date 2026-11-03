import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { Onboarding } from "./Onboarding";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { useLayout } from "../../store/layout";
import { useToast } from "../../store/toasts";
import { useGuide } from "../../store/guide";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.systemInfo.mockResolvedValue({
    system: { totalMb: 16384, logicalCores: 8 },
    budget: { budgetMb: 4096 },
    providers: ["cpu", "direct_ml"],
    onnxRuntime: "ONNX Runtime 1.24",
    resourceProfile: "balanced",
  } as never);
  commandsMock.listModels.mockResolvedValue([]);
  commandsMock.addSource.mockResolvedValue(3 as never);
  commandsMock.startImport.mockResolvedValue(11 as never);
  commandsMock.verifyModels.mockResolvedValue(12 as never);
  useLayout.setState({ theme: "system", language: "it", module: "jobs" });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

const renderIt = (onDone = vi.fn()) => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Onboarding onDone={onDone} />
    </QueryClientProvider>,
  );
  return onDone;
};
const click = (name: string | RegExp) => fireEvent.click(screen.getByRole("button", { name }));

it("Fine dell'introduzione: tema, cartella D:\\Foto e modelli, poi si apre la Libreria", async () => {
  const onDone = renderIt();
  expect(screen.getByRole("heading", { name: "Benvenuto" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("radio", { name: "Scuro" }));
  click("Continua");
  expect(useLayout.getState().theme).toBe("dark");

  expect(screen.getByRole("heading", { name: "Dove sono le tue foto?" })).toBeInTheDocument();
  // Senza cartella "Continua" spiega perché non è disponibile.
  const next = screen.getByRole("button", { name: "Continua" });
  expect(next).toHaveAttribute("aria-disabled", "true");
  fireEvent.click(next);
  expect(commandsMock.addSource).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("Cartella delle foto"), {
    target: { value: "D:\\Foto" },
  });
  click("Continua");
  await waitFor(() => expect(commandsMock.startImport).toHaveBeenCalledWith(3, null));
  expect(commandsMock.addSource).toHaveBeenCalledWith(
    expect.objectContaining({ path: "D:\\Foto", recursive: true }),
  );
  // Nessun salto ad Attività: un avviso permette di seguire l'importazione.
  expect(useToast.getState().notices[0]?.action?.label).toBe("Segui l'importazione");
  expect(useGuide.getState().done.import).toBe(true);

  expect(
    await screen.findByText(
      "Il computer ha 16 GB di memoria e 8 processori: useremo il profilo Bilanciato. Puoi cambiarlo in Impostazioni.",
    ),
  ).toBeInTheDocument();
  expect(screen.getByText(/La scheda video supporta l'accelerazione/)).toBeInTheDocument();
  click("Scarica i modelli e inizia");
  await waitFor(() => expect(onDone).toHaveBeenCalled());
  expect(commandsMock.verifyModels).toHaveBeenCalled();
  expect(commandsMock.completeOnboarding).toHaveBeenCalled();
  expect(useLayout.getState().module).toBe("library");
});

it("Download rimandato: ogni passo si salta e un avviso permette di scaricare i modelli", async () => {
  const onDone = renderIt();
  click("Lo faccio dopo");
  click("Lo faccio dopo");
  expect(screen.getByRole("heading", { name: "Funzioni AI" })).toBeInTheDocument();
  click("Lo faccio dopo");
  await waitFor(() => expect(onDone).toHaveBeenCalled());
  expect(commandsMock.addSource).not.toHaveBeenCalled();
  expect(commandsMock.verifyModels).not.toHaveBeenCalled();
  expect(commandsMock.completeOnboarding).toHaveBeenCalled();
  const notice = useToast.getState().notices.find((n) => n.key === "models-later");
  expect(notice?.message).toMatch(/Funzioni AI disattivate/);
  expect(notice?.duration).toBeNull();
  await notice!.action!.run();
  expect(commandsMock.verifyModels).toHaveBeenCalled();
});

it("Rivedere l'introduzione: nulla cambia finché un passo non è confermato", () => {
  useLayout.setState({ theme: "light" });
  renderIt();
  fireEvent.click(screen.getByRole("radio", { name: "Scuro" }));
  fireEvent.click(screen.getByRole("radio", { name: "English" }));
  click("Lo faccio dopo");
  expect(useLayout.getState().theme).toBe("light");
  expect(useLayout.getState().language).toBe("it");
  fireEvent.change(screen.getByLabelText("Cartella delle foto"), {
    target: { value: "D:\\Altro" },
  });
  click("Indietro");
  click("Lo faccio dopo");
  click("Lo faccio dopo");
  expect(commandsMock.addSource).not.toHaveBeenCalled();
});
