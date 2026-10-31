import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, it, expect, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { CatalogsPage } from "./CatalogsPage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(cleanup);
it("edits nested AND/OR rules, previews the count and saves a smart catalog", async () => {
  commandsMock.libraryCount.mockResolvedValue(42);
  commandsMock.listCatalogs.mockResolvedValue([]);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <CatalogsPage />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Nuovo catalogo"));
  fireEvent.change(screen.getByLabelText("Nome catalogo"), { target: { value: "Montagna" } });
  fireEvent.click(screen.getByLabelText("Smart"));
  fireEvent.click(screen.getByText("Aggiungi gruppo"));
  await screen.findByText("42 foto corrispondenti");
  fireEvent.click(screen.getByText("Salva"));
  await waitFor(() =>
    expect(commandsMock.saveCatalog).toHaveBeenCalledWith(
      null,
      "Montagna",
      "smart",
      expect.objectContaining({
        op: "and",
        rules: expect.arrayContaining([expect.objectContaining({ op: "or" })]),
      }),
    ),
  );
});
