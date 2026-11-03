import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
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
import { useLive } from "../../store/live";
import { useLayout } from "../../store/layout";
import { useUndo, useUndoHotkey } from "../../store/undo";
import { ToastHost } from "../../components/feedback/ToastHost";
import { useCatalogFeedback } from "./catalogActions";
import { LibraryHeader } from "./LibraryHeader";
import { SelectionBar } from "./SelectionBar";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup(A11Y_KNOWN.library));
beforeEach(() => {
  vi.clearAllMocks();
  useLive.setState({ jobs: {} });
  useUndo.setState({ last: null, busy: false });
});

/** Come in LibraryPage: la barra delle azioni prende il posto della barra della vista. */
function Bars() {
  useCatalogFeedback();
  const count = useLibrary((s) => selCount(s.selection));
  return count > 0 ? <SelectionBar /> : <LibraryHeader />;
}

function renderBars() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <Bars />
      <ToastHost />
    </QueryClientProvider>,
  );
}

const finish = (job: number, state = "completed", failed = 0) =>
  act(() =>
    useLive.setState({
      jobs: { [job]: { id: job, state, phases: [], failed } as never },
    }),
  );

async function openMenu(name: string | RegExp) {
  const user = userEvent.setup();
  screen.getByRole("button", { name }).focus();
  await user.keyboard("{Enter}");
  return user;
}

it("Selezione di foto: la barra delle azioni sostituisce la barra della vista, Esc la chiude", async () => {
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1, 2, 3]) });
  renderBars();
  const bar = screen.getByRole("toolbar", { name: "Azioni sulla selezione" });
  expect(within(bar).getByTestId("bulk-count")).toHaveTextContent("3 selezionate");
  for (const name of ["Tag", "Catalogo", "Persona", "Luogo", "Apri con", "Esporta", "Altro"])
    expect(within(bar).getByRole("button", { name })).toBeInTheDocument();
  fireEvent.keyDown(window, { key: "Escape" });
  expect(selCount(useLibrary.getState().selection)).toBe(0);
  expect(screen.queryByRole("toolbar")).not.toBeInTheDocument();
  expect(screen.getByRole("navigation", { name: "Percorso" })).toBeInTheDocument();
});

it("Aggiunta di un tag: scrivo «mare», Invio, avviso con Annulla operazione", async () => {
  const user = userEvent.setup();
  commandsMock.startBulk.mockResolvedValue(41 as never);
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([4, 5, 6]) });
  renderBars();
  await user.click(screen.getByRole("button", { name: "Tag" }));
  await user.type(await screen.findByRole("combobox", { name: "Cerca o scrivi un tag" }), "mare");
  await user.keyboard("{Enter}");
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [4, 5, 6] },
      { action: "add_tag", name: "mare" },
      false,
    ),
  );
  finish(41);
  const notice = await screen.findByText("Tag «mare» aggiunto a 3 foto");
  const toast = notice.closest('[role="status"]') as HTMLElement;
  await user.click(within(toast).getByRole("button", { name: "Annulla operazione" }));
  await waitFor(() => expect(commandsMock.undoBulk).toHaveBeenCalledTimes(1));
});

it("Azione che cambia modulo: Esporta apre il modulo Esporta", () => {
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1, 2]) });
  renderBars();
  fireEvent.click(screen.getByRole("button", { name: "Esporta" }));
  expect(useLayout.getState().module).toBe("organize");
});

it("seleziona i risultati non caricati, inverte; rimuovere dalla libreria chiede conferma", async () => {
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 2340, maxId: 2400 });
  renderBars();
  const user = await openMenu("Seleziona");
  await user.click(await screen.findByRole("menuitem", { name: /Seleziona tutte le foto/ }));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(2340));
  expect(screen.getByTestId("bulk-count")).toHaveTextContent("2340");
  await openMenu("Altro");
  await user.click(await screen.findByRole("menuitem", { name: "Inverti la selezione" }));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(0));

  act(() => useLibrary.setState({ selection: idsSelection([1, 2]) }));
  await openMenu("Altro");
  await user.click(await screen.findByRole("menuitem", { name: "Rimuovi dalla libreria…" }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent("Rimuovere 2 foto dalla libreria?");
  await user.click(within(dialog).getByRole("button", { name: "Annulla" }));
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
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
  const user = userEvent.setup();
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([4, 5, 6]) });
  renderBars();
  await user.click(screen.getByRole("button", { name: "Luogo" }));
  await user.type(await screen.findByRole("combobox", { name: "Scrivi un luogo" }), " Giappone ");
  await user.click(screen.getByRole("option", { name: "Imposta «Giappone»" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [4, 5, 6] },
      { action: "set_place", place: { name: "Giappone" } },
      false,
    ),
  );
  await user.click(screen.getByRole("button", { name: "Luogo" }));
  await user.click(await screen.findByRole("option", { name: "Ripristina il luogo automatico" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [4, 5, 6] },
      { action: "set_place", place: null },
      false,
    ),
  );
});

it("nuova voce in bulk: creata e assegnata alle 60 foto selezionate", async () => {
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
  renderBars();
  const user = await openMenu("Altro");
  await user.click(await screen.findByRole("menuitem", { name: "Categoria…" }));
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
  const filter = { ...emptyFilter(), text: "mare" };
  useLibrary.setState({ filter, selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 100_000, maxId: 100_000 });
  commandsMock.startBulk.mockResolvedValue(9 as never);
  renderBars();
  const user = await openMenu("Seleziona");
  await user.click(await screen.findByRole("menuitem", { name: /Seleziona tutte le foto/ }));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(100_000));
  // Ctrl+clic su tre miniature: diventano eccezioni.
  act(() => useLibrary.setState((s) => ({ selection: selSet(s.selection, [7, 8, 9], false) })));
  await waitFor(() => expect(screen.getByTestId("bulk-count")).toHaveTextContent("99997"));
  await user.click(screen.getByRole("button", { name: "Tag" }));
  await user.type(await screen.findByRole("combobox"), "Spiaggia{Enter}");
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "filter", filter, maxId: 100_000, except: [7, 8, 9] },
      { action: "add_tag", name: "Spiaggia" },
      false,
    ),
  );
  // Nessun elenco di 100.000 id è mai arrivato nell'interfaccia.
  expect(JSON.stringify(useLibrary.getState().selection).length).toBeLessThan(2000);
  // "Apri con" lavora su id espliciti: con una selezione per filtro spiega perché non c'è.
  expect(screen.getByRole("button", { name: "Apri con" })).toHaveAttribute("aria-disabled", "true");
});

it("Risultati cambiati dopo la selezione: l'azione resta limitata agli elementi di allora", async () => {
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
  commandsMock.librarySelection.mockResolvedValue({ count: 500, maxId: 500 });
  commandsMock.startBulk.mockResolvedValue(10 as never);
  renderBars();
  const user = await openMenu("Seleziona");
  await user.click(await screen.findByRole("menuitem", { name: /Seleziona tutte le foto/ }));
  await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(500));
  // Un import aggiunge foto: la base di una nuova selezione cambierebbe, quella attuale no.
  commandsMock.librarySelection.mockResolvedValue({ count: 800, maxId: 800 });
  await user.click(screen.getByRole("button", { name: "Tag" }));
  await user.type(await screen.findByRole("combobox"), "X{Enter}");
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "filter", maxId: 500 }),
      { action: "add_tag", name: "X" },
      false,
    ),
  );
});

it("reinserisce dalla vista escluse senza conferma", async () => {
  useLibrary.setState({
    filter: { ...emptyFilter(), excludedFrom: 3 },
    selection: idsSelection([7]),
  });
  commandsMock.listCatalogs.mockResolvedValue([
    {
      catalog: { id: 3, name: "Viaggio", kind: "manual", rules: null, createdAt: 0 },
      items: 10,
      excluded: 1,
    },
  ] as never);
  commandsMock.startBulk.mockResolvedValue(901);
  renderBars();
  fireEvent.click(await screen.findByRole("button", { name: "Reinserisci nel catalogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "restore_to_catalog", catalog: 3 },
      false,
    ),
  );
  finish(901);
  await screen.findByRole("status");
});

it("assegnare una persona chiede conferma perché non si annulla da qui", async () => {
  const user = userEvent.setup();
  commandsMock.listPeople.mockResolvedValue([
    { id: 5, name: "Giulia", identityUid: null, photos: 12, faces: 20, cover: null },
  ] as never);
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1, 2]) });
  renderBars();
  await user.click(screen.getByRole("button", { name: "Persona" }));
  await user.click(await screen.findByRole("option", { name: /Giulia/ }));
  const dialog = await screen.findByRole("alertdialog");
  await user.click(within(dialog).getByRole("button", { name: "Assegna a Giulia" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [1, 2] },
      { action: "assign_person", person: 5 },
      false,
    ),
  );
});

function UndoKeys() {
  useUndoHotkey();
  return null;
}

it("Annullamento dopo aver cambiato pagina: Ctrl+Z annulla l'ultima operazione della Libreria", async () => {
  const user = userEvent.setup();
  commandsMock.startBulk.mockResolvedValue(77 as never);
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([1, 2]) });
  const view = render(
    <QueryClientProvider client={new QueryClient()}>
      <UndoKeys />
      <Bars />
    </QueryClientProvider>,
  );
  await user.click(screen.getByRole("button", { name: "Tag" }));
  await user.type(await screen.findByRole("combobox"), "mare{Enter}");
  await waitFor(() => expect(commandsMock.startBulk).toHaveBeenCalled());
  finish(77);
  await waitFor(() => expect(useUndo.getState().last?.label).toBe("Tag «mare» aggiunto a 2 foto"));
  // Si lascia la Libreria: resta solo il gestore globale di Ctrl+Z.
  view.rerender(
    <QueryClientProvider client={new QueryClient()}>
      <UndoKeys />
    </QueryClientProvider>,
  );
  fireEvent.keyDown(window, { key: "z", ctrlKey: true });
  await waitFor(() => expect(commandsMock.undoBulk).toHaveBeenCalledTimes(1));
});
