import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, it, expect, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { CatalogsPage } from "./CatalogsPage";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useToast } from "../../store/toasts";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup(A11Y_KNOWN.none));
beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.libraryFacets.mockResolvedValue(null);
  commandsMock.getTaxonomy.mockResolvedValue({
    version: 1,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [{ id: "paesaggio", label: "Paesaggio", enabled: true, prompts: [] }],
      },
    ],
  } as never);
});

const renderPage = (client = new QueryClient()) =>
  render(
    <QueryClientProvider client={client}>
      <CatalogsPage />
    </QueryClientProvider>,
  );

const summary = (id: number, name: string, kind: "smart" | "manual", items: number, excluded = 0) =>
  ({
    catalog: { id, name, kind, rules: null, createdAt: 0 },
    items,
    excluded,
  }) as never;

/** Sceglie un'opzione per etichetta (i valori delle opzioni non sono gli id interni). */
function choose(select: HTMLElement, name: string) {
  const option = within(select).getByRole("option", { name }) as HTMLOptionElement;
  fireEvent.change(select, { target: { value: option.value } });
}

it("Errore di caricamento: offers retry instead of an empty catalog list", async () => {
  commandsMock.listCatalogs.mockRejectedValueOnce(new Error("Catalog unavailable"));
  renderPage(new QueryClient({ defaultOptions: { queries: { retry: false } } }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Catalog unavailable");
  commandsMock.listCatalogs.mockResolvedValue([]);
  fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
});

it("stato vuoto con il comando per creare un catalogo", async () => {
  commandsMock.listCatalogs.mockResolvedValue([]);
  renderPage();
  expect(await screen.findByRole("heading", { name: "Nessun catalogo" })).toBeInTheDocument();
  const create = screen.getAllByRole("button", { name: "Nuovo catalogo" });
  fireEvent.click(create[create.length - 1]);
  expect(screen.getByLabelText("Nome catalogo")).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "Nessun catalogo" })).toBeNull();
});

it("Catalogo con esclusioni nell'elenco: tipo, foto ed escluse con accesso diretto", async () => {
  commandsMock.listCatalogs.mockResolvedValue([
    summary(1, "Viaggio Toscana", "smart", 12312, 6),
    summary(2, "Matrimonio", "manual", 40),
  ]);
  renderPage();
  const card = await screen.findByRole("region", { name: "Viaggio Toscana" });
  expect(card).toHaveTextContent("Smart");
  expect(card).toHaveTextContent("12.312 foto");
  const manual = screen.getByRole("region", { name: "Matrimonio" });
  expect(manual).toHaveTextContent("Manuale");
  expect(within(manual).queryByText(/escluse/)).toBeNull();
  fireEvent.click(within(card).getByRole("button", { name: "6 escluse" }));
  expect(useLibrary.getState().filter.excludedFrom).toBe(1);
  expect(useLibrary.getState().sort).toBe("excluded_at");
  expect(useLayout.getState().module).toBe("library");
});

it("eliminazione con ConfirmDialog: nessun window.confirm, avviso a operazione riuscita", async () => {
  const confirm = vi.spyOn(window, "confirm");
  commandsMock.listCatalogs.mockResolvedValue([summary(3, "Panorami", "smart", 12)]);
  renderPage();
  const card = await screen.findByRole("region", { name: "Panorami" });
  fireEvent.click(within(card).getByRole("button", { name: "Elimina catalogo" }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent("Eliminare il catalogo «Panorami»?");
  expect(dialog).toHaveTextContent("Le 12 foto del catalogo restano nella libreria");
  expect(commandsMock.deleteCatalog).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Elimina catalogo" }));
  await waitFor(() => expect(commandsMock.deleteCatalog).toHaveBeenCalledWith(3, true));
  await waitFor(() =>
    expect(useToast.getState().notices.map((n) => n.message)).toContain(
      "Catalogo «Panorami» eliminato",
    ),
  );
  expect(confirm).not.toHaveBeenCalled();
});

it("edits nested AND/OR rules, previews the count and saves a smart catalog", async () => {
  commandsMock.libraryCount.mockResolvedValue(42);
  commandsMock.listCatalogs.mockResolvedValue([]);
  renderPage();
  fireEvent.click((await screen.findAllByRole("button", { name: "Nuovo catalogo" }))[0]);
  fireEvent.change(screen.getByLabelText("Nome catalogo"), { target: { value: "Montagna" } });
  fireEvent.click(screen.getByRole("radio", { name: "Smart" }));
  fireEvent.click(screen.getByRole("button", { name: "Aggiungi gruppo di condizioni" }));
  await screen.findByText("42 foto corrispondenti");
  // Riepilogo in una frase, aggiornato con la regola (il gruppo annidato ha una condizione).
  expect(
    screen.getByText("Foto con tipo soggetto Paesaggio, con tipo soggetto Paesaggio"),
  ).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
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
  renderPage();
  fireEvent.click((await screen.findAllByRole("button", { name: "Nuovo catalogo" }))[0]);
  fireEvent.change(screen.getByLabelText("Nome catalogo"), { target: { value: "Compleanni" } });
  fireEvent.click(screen.getByRole("radio", { name: "Smart" }));
  await screen.findByRole("option", { name: "Evento" });
  choose(screen.getAllByLabelText("Campo")[0], "Evento");
  const value = await screen.findByRole("combobox", { name: "Valore" });
  choose(value, "Compleanno");
  expect(screen.getByText("Foto con evento Compleanno")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
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
