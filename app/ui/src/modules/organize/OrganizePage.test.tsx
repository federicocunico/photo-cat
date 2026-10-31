import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { OrganizePage } from "./OrganizePage";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

const renderPage = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <OrganizePage />
    </QueryClientProvider>,
  );

it("esporta in CSV con separatore, BOM e campi scelti", async () => {
  renderPage();
  fireEvent.change(screen.getByLabelText("Ambito"), { target: { value: "library" } });
  await screen.findByLabelText("Persone");
  fireEvent.click(screen.getByLabelText("Persone"));
  fireEvent.change(screen.getByLabelText("File di destinazione"), {
    target: { value: "D:\\Export\\catalogo.csv" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Esporta" }));
  await waitFor(() =>
    expect(commandsMock.startExport).toHaveBeenCalledWith({
      scope: { scope: "library" },
      destination: "D:\\Export\\catalogo.csv",
      format: "csv",
      separator: ";",
      bom: true,
      fields: ["path", "tags"],
    }),
  );
  expect(await screen.findByRole("status")).toHaveTextContent("Job #1 accodato.");
});

it("mostra anteprima con conteggi e spazio, propone l'alternativa e solo dopo avvia", async () => {
  commandsMock.organizePreview.mockResolvedValue({
    entries: [],
    folders: { "Kyoto/2025": 120, "Senza luogo/2025": 3 },
    files: 123,
    bytes: 3 * 1024 * 1024 * 1024,
    conflicts: 2,
  } as never);
  commandsMock.organizePreflight
    .mockResolvedValueOnce({
      available: false,
      alternative: "hardlink",
      developerMode: false,
      reason: "symlink_privilege",
      message: "",
    } as never)
    .mockResolvedValue({
      available: true,
      alternative: "hardlink",
      developerMode: false,
      reason: null,
      message: "",
    } as never);
  commandsMock.startOrganize.mockResolvedValue(9 as never);
  renderPage();
  fireEvent.change(screen.getByLabelText("Ambito"), { target: { value: "library" } });
  fireEvent.change(screen.getByLabelText("Cartella di destinazione"), {
    target: { value: "E:\\Organizzate" },
  });
  fireEvent.change(screen.getByLabelText("Metodo"), { target: { value: "symlink" } });
  const start = screen.getByRole("button", { name: "Crea cartelle" });
  expect(start).toBeDisabled();

  fireEvent.click(screen.getByRole("button", { name: "Anteprima" }));
  expect(await screen.findByText(/123 file · 2 conflitti/)).toBeInTheDocument();
  expect(screen.getByText("Kyoto/2025 · 120")).toBeInTheDocument();
  expect(screen.getByText("Senza luogo/2025 · 3")).toBeInTheDocument();
  expect(commandsMock.organizePreview).toHaveBeenCalledWith(
    expect.objectContaining({ levels: ["place", "year"], method: "symlink" }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Verifica metodo" }));
  expect(await screen.findByText(/Modalità sviluppatore di Windows/)).toBeInTheDocument();
  fireEvent.click(await screen.findByRole("button", { name: "Hard link" }));
  expect(screen.getByLabelText("Metodo")).toHaveValue("hardlink");
  // Cambiare metodo invalida anteprima e verifica: vanno ripetute.
  expect(start).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Anteprima" }));
  await screen.findByText(/123 file/);
  fireEvent.click(screen.getByRole("button", { name: "Verifica metodo" }));
  await waitFor(() => expect(start).toBeEnabled());
  fireEvent.click(start);
  await waitFor(() =>
    expect(commandsMock.startOrganize).toHaveBeenCalledWith(
      expect.objectContaining({ destination: "E:\\Organizzate", method: "hardlink" }),
    ),
  );
});
