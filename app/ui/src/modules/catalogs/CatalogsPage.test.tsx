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
it("i gruppi solo manuali compaiono nell'editor dei cataloghi smart", async () => {
  commandsMock.libraryCount.mockResolvedValue(3);
  commandsMock.listCatalogs.mockResolvedValue([]);
  commandsMock.getTaxonomy.mockResolvedValue({
    version: 5,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [{ id: "paesaggio", label: "Paesaggio", enabled: true, prompts: ["x"] }],
      },
      {
        id: "evento",
        label: "Evento",
        threshold: 0.25,
        categories: [
          { id: "compleanno", label: "Compleanno", enabled: true, prompts: [] },
          { id: "laurea", label: "Laurea", enabled: true, prompts: [] },
        ],
      },
    ],
  } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <CatalogsPage />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Nuovo catalogo"));
  fireEvent.change(screen.getByLabelText("Nome catalogo"), { target: { value: "Compleanni" } });
  fireEvent.click(screen.getByLabelText("Smart"));
  await screen.findByRole("option", { name: "Evento" });
  fireEvent.change(screen.getAllByLabelText("Campo")[0], { target: { value: "label.evento" } });
  const value = await screen.findByRole("combobox", { name: "Valore" });
  expect(screen.getByRole("option", { name: "Compleanno" })).toBeInTheDocument();
  fireEvent.change(value, { target: { value: "compleanno" } });
  fireEvent.click(screen.getByText("Salva"));
  await waitFor(() =>
    expect(commandsMock.saveCatalog).toHaveBeenCalledWith(
      null,
      "Compleanni",
      "smart",
      expect.objectContaining({
        rules: [{ field: "label.evento", cmp: "eq", value: "compleanno" }],
      }),
    ),
  );
});
