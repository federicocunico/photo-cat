import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { ImportPage } from "./ImportPage";
import { splitPatterns } from "./formats";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

beforeEach(() => {
  vi.clearAllMocks();
  Object.assign(commandsMock, {
    listSources: vi.fn(async () => []),
    addSource: vi.fn(async () => 9),
    startImport: vi.fn(async () => 1),
    updateSource: vi.fn(async () => null),
    removeSource: vi.fn(async () => null),
  });
});
afterEach(cleanup);

const renderPage = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <ImportPage />
    </QueryClientProvider>,
  );

describe("Importa", () => {
  it("crea la sorgente con le opzioni scelte e avvia l'import", async () => {
    renderPage();
    const start = screen.getByRole("button", { name: "Avvia importazione" });
    expect(start).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "D:\\Foto\\2024" } });
    fireEvent.click(screen.getByLabelText("Monitora la cartella per nuovi file"));
    fireEvent.click(screen.getByLabelText("RAW (RAF, CR2, CR3, NEF, ARW, DNG…)"));
    fireEvent.click(screen.getByLabelText("PNG"));
    fireEvent.change(screen.getByLabelText("Ignora percorsi che corrispondono a"), {
      target: { value: "**/Cestino/** ; *.tmp" },
    });
    fireEvent.click(start);
    await waitFor(() => expect(commandsMock.startImport).toHaveBeenCalledWith(9));
    const arg = commandsMock.addSource.mock.calls[0][0] as unknown as {
      path: string;
      recursive: boolean;
      watch: boolean;
      includeFormats: string[];
      ignorePatterns: string[];
    };
    expect(arg.path).toBe("D:\\Foto\\2024");
    expect(arg.recursive).toBe(true);
    expect(arg.watch).toBe(true);
    expect(arg.includeFormats).toContain("nef");
    expect(arg.includeFormats).not.toContain("png");
    expect(arg.ignorePatterns).toEqual(["**/Cestino/**", "*.tmp"]);
    expect(useLayout.getState().module).toBe("jobs");
  });

  it("mostra l'errore del backend (cartella inesistente)", async () => {
    commandsMock.addSource.mockRejectedValueOnce({
      code: "NOT_FOUND",
      detail: "cartella non trovata: X:\\nope",
    } as never);
    renderPage();
    fireEvent.change(screen.getByLabelText("Cartella"), { target: { value: "X:\\nope" } });
    fireEvent.click(screen.getByRole("button", { name: "Avvia importazione" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("cartella non trovata");
    expect(commandsMock.startImport).not.toHaveBeenCalled();
  });

  it("separa i pattern da ignorare", () => {
    expect(splitPatterns(" a ; b\nc;; ")).toEqual(["a", "b", "c"]);
  });
});
