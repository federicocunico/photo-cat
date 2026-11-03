import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, it, expect, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import {
  emptyFilter,
  idsSelection,
  rectangleSelect,
  selCount,
  selSet,
  useLibrary,
} from "../../store/library";
import { CatalogFeedback } from "./catalogActions";
import { useLive } from "../../store/live";
import { ToastHost } from "../../components/feedback/ToastHost";
import { BulkBar } from "./BulkBar";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup(A11Y_KNOWN.library));
it("selects unseen filtered results, inverts and confirms destructive edits", async () => {
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 2340, maxId: 2400 });
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(2340));
  expect(screen.getByTestId("bulk-count")).toHaveTextContent("2340");
  fireEvent.click(screen.getByText("Inverti"));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(0));
  useLibrary.setState({ selection: idsSelection([1, 2]) });
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "remove_library" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() => expect(confirm).toHaveBeenCalledWith(expect.stringContaining("2 foto")));
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
  confirm.mockRestore();
});
it("rectangle selects rows independently of the virtual DOM", () => {
  const ids = Array.from({ length: 100000 }, (_, i) => i + 1);
  expect(
    rectangleSelect(
      ids,
      { x: 16, y: 216, w: 200, h: 99 },
      { cols: 4, cell: 90, rowHeight: 100, gap: 10, pad: 16 },
    ),
  ).toEqual([9, 10]);
});
it("imposta e ripristina il luogo sulla selezione", async () => {
  vi.clearAllMocks();
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([4, 5, 6]) });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "set_place" } });
  expect(screen.getByText("Applica")).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: " Giappone " } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [4, 5, 6] },
      { action: "set_place", place: { name: "Giappone" } },
      false,
    ),
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "reset_place" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [4, 5, 6] },
      { action: "set_place", place: null },
      false,
    ),
  );
});
it("nuova voce in bulk: creata e assegnata alle 60 foto selezionate", async () => {
  vi.clearAllMocks();
  const ids = Array.from({ length: 60 }, (_, i) => i + 1);
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection(ids) });
  commandsMock.getTaxonomy.mockResolvedValue({
    version: 4,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [{ id: "fiori", label: "Fiori", enabled: true, prompts: ["flowers"] }],
      },
      {
        id: "evento",
        label: "Evento",
        threshold: 0.25,
        categories: [{ id: "laurea", label: "Laurea", enabled: true, prompts: [] }],
      },
    ],
  } as never);
  commandsMock.addCategory.mockResolvedValue({
    group: "evento",
    id: "gita_scolastica",
    label: "Gita scolastica",
    created: true,
    job: 12,
  } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "set_label" } });
  await screen.findByRole("option", { name: "Evento" });
  fireEvent.change(screen.getByLabelText("Campo"), { target: { value: "evento" } });
  expect(screen.getByRole("option", { name: "Laurea" })).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: "__new__" } });
  fireEvent.click(screen.getByText("Conferma"));
  expect(await screen.findByRole("alert")).toHaveTextContent("Il nome è obbligatorio");
  fireEvent.change(screen.getByLabelText("Nome della nuova voce"), {
    target: { value: "Gita scolastica" },
  });
  fireEvent.click(screen.getByText("Conferma"));
  await waitFor(() =>
    expect(commandsMock.addCategory).toHaveBeenCalledWith("evento", "Gita scolastica", [], {
      kind: "ids",
      ids,
    }),
  );
  expect(commandsMock.addCategory).toHaveBeenCalledTimes(1);
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
});
it("Seleziona tutto su una libreria grande: 100.000 meno 3, tag applicato al filtro con eccezioni", async () => {
  vi.clearAllMocks();
  const filter = { ...emptyFilter(), text: "mare" };
  useLibrary.setState({ filter, selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 100_000, maxId: 100_000 });
  commandsMock.startBulk.mockResolvedValue(9 as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(100_000));
  // Ctrl+clic su tre miniature: diventano eccezioni.
  useLibrary.setState((s) => ({ selection: selSet(s.selection, [7, 8, 9], false) }));
  await waitFor(() => expect(screen.getByTestId("bulk-count")).toHaveTextContent("99997"));
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: "Spiaggia" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "filter", filter, maxId: 100_000, except: [7, 8, 9] },
      { action: "add_tag", name: "Spiaggia" },
      false,
    ),
  );
  // Nessun elenco di 100.000 id è mai arrivato nell'interfaccia.
  expect(JSON.stringify(useLibrary.getState().selection).length).toBeLessThan(2000);
});
it("Risultati cambiati dopo la selezione: l'azione resta limitata agli elementi di allora", async () => {
  vi.clearAllMocks();
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 500, maxId: 500 });
  commandsMock.startBulk.mockResolvedValue(10 as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(500));
  // Un import aggiunge foto: la base di una nuova selezione cambierebbe, quella attuale no.
  commandsMock.librarySelection.mockResolvedValue({ count: 800, maxId: 800 });
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: "X" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "filter", maxId: 500 }),
      { action: "add_tag", name: "X" },
      false,
    ),
  );
});

it("reinserisce dalla vista escluse senza conferma", async () => {
  vi.clearAllMocks();
  useLibrary.setState({
    filter: { ...emptyFilter(), excludedFrom: 3 },
    selection: idsSelection([7]),
  });
  commandsMock.startBulk.mockResolvedValue(901);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
      <CatalogFeedback />
      <ToastHost />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Reinserisci nel catalogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "restore_to_catalog", catalog: 3 },
      false,
    ),
  );
  act(() =>
    useLive.setState({
      jobs: { 901: { id: 901, state: "completed", phases: [], failed: 0 } as never },
    }),
  );
  await screen.findByRole("status");
});
