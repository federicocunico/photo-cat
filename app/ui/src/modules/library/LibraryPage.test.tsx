import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import "../../i18n";
import "../../test/layoutMock";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import {
  clickSelect,
  emptyFilter,
  idsSelection,
  selCount,
  selIds,
  useLibrary,
} from "../../store/library";
import { useLayout } from "../../store/layout";
import { LibraryPage } from "./LibraryPage";
import { useLive } from "../../store/live";
import { useFloating } from "../../store/floating";
import i18n from "../../i18n";
import type { GridItem, SourceSummary } from "../../bindings";
import { deferred } from "../../test/deferred";
import { ToastHost } from "../../components/feedback/ToastHost";
import { LibraryStatus } from "./LibraryStatus";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

it("Primo caricamento della griglia: placeholders and calculating count, never an empty library", () => {
  commandsMock.libraryPage.mockImplementationOnce(() => new Promise(() => {}));
  commandsMock.libraryCount.mockImplementation(() => new Promise(() => {}));
  render(
    <QueryClientProvider client={new QueryClient()}>
      <LibraryPage />
      <LibraryStatus />
    </QueryClientProvider>,
  );
  expect(document.querySelectorAll(".feedback-skeleton-thumbnail")).toHaveLength(24);
  expect(screen.getByTestId("status-count")).toHaveTextContent("in calcolo…");
  expect(screen.queryByText(i18n.t("library.empty"))).not.toBeInTheDocument();
});

it("Aggiornamento in background: keeps and dims the previous photos until the new filter completes", async () => {
  const pending = deferred<{ items: GridItem[]; next: null }>();
  renderPage();
  await screen.findByTestId("thumb-1");
  commandsMock.libraryPage.mockImplementationOnce(() => pending.promise);
  act(() => useLibrary.getState().setFilter({ text: "new filter" }));
  await waitFor(() =>
    expect(screen.getByTestId("grid-scroll")).toHaveAttribute("data-refreshing", "true"),
  );
  expect(screen.getByTestId("thumb-1")).toBeInTheDocument();
  await act(async () => pending.resolve({ items: [item(99)], next: null }));
  expect(await screen.findByTestId("thumb-99")).toBeInTheDocument();
  expect(screen.queryByTestId("thumb-1")).not.toBeInTheDocument();
});

it("Sidebar loading has feedback and Inspector ignores a double click on an async action", async () => {
  const pendingSources = deferred<SourceSummary[]>();
  commandsMock.listSources.mockImplementationOnce(() => pendingSources.promise);
  renderPage();
  expect(screen.getAllByRole("status", { name: "Caricamento…" }).length).toBeGreaterThan(0);
  fireEvent.click(await screen.findByTestId("thumb-1"));
  const inspector = await screen.findByTestId("inspector");
  const pending = deferred<number>();
  commandsMock.startBulk.mockImplementationOnce(() => pending.promise);
  fireEvent.change(within(inspector).getByLabelText(i18n.t("library.inspector.tag")), {
    target: { value: "Test" },
  });
  const button = within(inspector).getByRole("button", { name: i18n.t("bulk.actions.add_tag") });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(commandsMock.startBulk).toHaveBeenCalledOnce();
  expect(button).toHaveAttribute("aria-busy", "true");
  await act(async () => {
    pending.resolve(1);
    pendingSources.resolve([]);
  });
});

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

/** Lista di un filtro con tutte le voci. */
const list = (values: { value: string; count: number }[], total = values.length) => ({
  values,
  total,
});

const item = (id: number): GridItem => ({
  id,
  excludedAt: null,
  name: `IMG_${String(id).padStart(4, "0")}.JPG`,
  path: `D:\\Foto\\IMG_${id}.JPG`,
  thumb: id % 2 ? `abcdef${id}0000` : null,
  capturedAt: "2023-07-14 10:22:01",
  width: 6000,
  height: 4000,
  status: id === 3 ? "missing" : "ok",
  faces: id === 2 ? 3 : 0,
  analyzed: id % 3 === 0,
  tags: id === 1 ? ["Tramonti"] : [],
  similarGroup: null,
  similarCover: false,
  similarCount: null,
});

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLayout.setState({ leftOpen: true, rightOpen: true });
  useLibrary.setState({
    filter: emptyFilter(),
    selection: idsSelection(),
    focused: null,
    scrollTop: 0,
    thumbSize: 3,
  });
  Object.assign(commandsMock, {
    libraryPage: vi.fn(async () => ({
      items: Array.from({ length: 12 }, (_, i) => item(i + 1)),
      next: null,
    })),
    libraryCount: vi.fn(async () => 12),
    tagOverview: vi.fn(async () => ({
      tags: [{ name: "Tramonti", count: 1, color: "yellow", sets: [] }],
      sets: [],
      activeSet: null,
      palette: [],
    })),
    getTaxonomy: vi.fn(async () => ({ version: 1, groups: [] })),
    libraryFacets: vi.fn(async () => ({
      labels: [
        [
          "subject",
          list([
            { value: "paesaggio", count: 7 },
            { value: "ritratto", count: 5 },
          ]),
        ],
      ],
      places: list([{ value: "Siena", count: 4 }]),
      years: list([{ value: "2023", count: 12 }]),
      formats: list([{ value: "jpg", count: 12 }]),
      people: list([]),
      tags: list([]),
    })),
    listSources: vi.fn(async () => [
      {
        source: {
          id: 1,
          path: "D:\\Foto",
          recursive: true,
          includeFormats: [],
          ignorePatterns: [],
          watch: false,
          createdAt: 0,
        },
        items: 12,
        missing: 1,
      },
    ]),
    listCatalogs: vi.fn(async () => [
      {
        catalog: { id: 5, name: "Ritratti di Giulia", kind: "smart", rules: null, createdAt: 0 },
        items: 4,
      },
    ]),
    mediaDetail: vi.fn(async (id: number) => ({
      item: {
        ...item(id),
        kind: "photo",
        sourceId: 1,
        ext: "jpg",
        size: 8_808_038,
        mtime: 0,
        quickHash: null,
        contentHash: null,
        addedAt: 0,
        updatedAt: 0,
      },
      tech: {
        format: "JPEG",
        width: 6000,
        height: 4000,
        dpiX: 300,
        dpiY: 300,
        bitDepth: 8,
        colorSpace: "sRGB",
        orientation: 1,
      },
      dates: {
        capturedAt: "2023-08-14 18:42:00",
        capturedSource: "exif_original",
        modifiedAt: "2023-09-02 10:05:00",
      },
      exif: null,
      location: { name: "Val d'Orcia", country: null, lat: null, lon: null, source: "gps" },
      labels: [{ group: "subject", value: "paesaggio", score: 0.82, origin: "auto" }],
      peopleCount: 2,
      people: ["Giulia"],
      faces: [
        {
          id: 11,
          bbox: [0.1, 0.1, 0.2, 0.2],
          personId: 4,
          personName: "Giulia",
          identityUid: "g",
          ignored: false,
          discarded: false,
        },
      ],
      tags: ["tramonto"],
      catalogs: [],
      duplicates: [],
    })),
  });
});
afterEach(checkedCleanup(A11Y_KNOWN.library));

function renderPage() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <LibraryPage />
      <ToastHost />
    </QueryClientProvider>,
  );
}

describe("selezione", () => {
  const order = [1, 2, 3, 4, 5];
  it("clic, Ctrl e Shift", () => {
    expect(clickSelect(order, [1], 1, 3, { ctrl: false, shift: false })).toEqual([3]);
    expect(clickSelect(order, [1], 1, 3, { ctrl: true, shift: false })).toEqual([1, 3]);
    expect(clickSelect(order, [1, 3], 3, 3, { ctrl: true, shift: false })).toEqual([1]);
    expect(clickSelect(order, [2], 2, 4, { ctrl: false, shift: true })).toEqual([2, 3, 4]);
    expect(clickSelect(order, [5], 4, 2, { ctrl: true, shift: true })).toEqual([5, 2, 3, 4]);
  });
});

describe("Libreria", () => {
  it("griglia, badge, selezione e ispettore", async () => {
    renderPage();
    const first = await screen.findByTestId("thumb-1");
    expect(first.querySelector("img")).toHaveAttribute("src", "http://thumb.localhost/abcdef10000");
    expect(screen.getByTestId("thumb-3")).toHaveTextContent("Mancante");
    expect(screen.getByTestId("thumb-2")).toHaveTextContent("3");
    // Chip colorato del tag sulla miniatura.
    expect(screen.getByLabelText("Tag Tramonti")).toHaveAttribute("data-tag-color", "yellow");

    fireEvent.click(first);
    await screen.findByTestId("inspector");
    const insp = screen.getByTestId("inspector");
    expect(insp).toHaveTextContent("JPEG");
    expect(insp).toHaveTextContent("6000 × 4000");
    expect(insp).toHaveTextContent("300 × 300 DPI");
    expect(insp).toHaveTextContent("8 bit per canale");
    expect(insp).toHaveTextContent("8,4 MB");
    expect(insp).toHaveTextContent("Val d'Orcia");
    expect(insp).toHaveTextContent("GPS EXIF");
    expect(insp).toHaveTextContent("Tipo soggetto: Paesaggio");
    expect(insp).toHaveTextContent("0,82");
    expect(insp).toHaveTextContent("Giulia");
    expect(insp).toHaveTextContent("tramonto");
    expect(commandsMock.mediaDetail).toHaveBeenCalledWith(1);

    fireEvent.click(screen.getByTestId("thumb-4"), { shiftKey: true });
    expect(selIds(useLibrary.getState().selection)).toEqual([1, 2, 3, 4]);
    fireEvent.click(screen.getByTestId("thumb-2"), { ctrlKey: true });
    expect(selIds(useLibrary.getState().selection)).toEqual([1, 3, 4]);
  });

  it("faccette e chip modificano il filtro della query", async () => {
    renderPage();
    await screen.findByTestId("thumb-1");
    expect(await screen.findByText("Tipo soggetto")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: /Paesaggio/ }));
    await waitFor(() =>
      expect(commandsMock.libraryPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ labels: [{ group: "subject", values: ["paesaggio"] }] }),
        "captured_desc",
        null,
        300,
      ),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: /Siena/ }));
    await waitFor(() => expect(useLibrary.getState().filter.places).toEqual(["Siena"]));
    fireEvent.click(screen.getByRole("button", { name: "Rimuovi filtro Paesaggio" }));
    await waitFor(() => expect(useLibrary.getState().filter.labels).toEqual([]));
    expect(useLibrary.getState().filter.places).toEqual(["Siena"]);

    // Sorgenti e cataloghi nel pannello sinistro.
    expect(screen.getByRole("button", { name: /^D:\\Foto/ })).toHaveTextContent("12");
    fireEvent.click(screen.getByRole("button", { name: /Ritratti di Giulia/ }));
    expect(useLibrary.getState().filter.catalogId).toBe(5);
  });

  it("ordinamento e dimensione miniature", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByTestId("thumb-1");
    screen.getByRole("button", { name: "Ordina: Data acquisizione ↓" }).focus();
    await user.keyboard("{Enter}");
    await user.click(await screen.findByRole("menuitem", { name: "Nome file" }));
    await waitFor(() =>
      expect(commandsMock.libraryPage).toHaveBeenLastCalledWith(
        expect.anything(),
        "name_asc",
        null,
        300,
      ),
    );
    await user.click(screen.getByRole("button", { name: "Vista" }));
    fireEvent.change(await screen.findByLabelText("Dimensione miniature"), {
      target: { value: "5" },
    });
    expect(useLibrary.getState().thumbSize).toBe(5);
    // Miniature grandi: meno colonne, ciascuna almeno 260 px e allargata a riempire la riga.
    expect(parseInt(screen.getByTestId("thumb-1").style.width)).toBeGreaterThanOrEqual(260);
  });
});

describe("cartelle, luogo e Apri con", () => {
  const trip = "D:\\Foto\\Viaggio Giappone";
  beforeEach(() => {
    useLibrary.setState({ sort: "captured_desc", folderRecursive: true });
    commandsMock.listFolders.mockImplementation(async (_source, parent) =>
      parent === null
        ? [{ path: trip, name: "Viaggio Giappone", items: 5, hasChildren: true }]
        : parent === trip
          ? [
              { path: `${trip}\\D7000`, name: "D7000", items: 3, hasChildren: false },
              { path: `${trip}\\Telefono`, name: "Telefono", items: 2, hasChildren: false },
            ]
          : [],
    );
    commandsMock.getConfig.mockResolvedValue({
      external_apps: [{ id: "lr", name: "Lightroom", path: "C:\\LR\\Lightroom.exe", args: [] }],
      ui: { theme: "dark", language: "it", double_click: "lr" },
    } as never);
  });

  it("albero delle cartelle, filtro e Seleziona tutto in una cartella", async () => {
    renderPage();
    await screen.findByTestId("thumb-1");
    fireEvent.click(await screen.findByRole("button", { name: "Mostra le cartelle di D:\\Foto" }));
    const tripRow = await screen.findByRole("button", { name: /^Viaggio Giappone/ });
    expect(tripRow).toHaveTextContent("5");
    fireEvent.click(screen.getByRole("button", { name: "Mostra le cartelle di Viaggio Giappone" }));
    const d7000 = await screen.findByRole("button", { name: /^D7000/ });
    expect(d7000).toHaveTextContent("3");
    expect(commandsMock.listFolders).toHaveBeenCalledWith(1, trip);

    fireEvent.click(d7000);
    expect(useLibrary.getState().filter.folder).toEqual({
      path: `${trip}\\D7000`,
      recursive: true,
    });
    fireEvent.click(screen.getByLabelText("Includi sottocartelle"));
    expect(useLibrary.getState().filter.folder?.recursive).toBe(false);
    await waitFor(() =>
      expect(commandsMock.libraryPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ folder: { path: `${trip}\\D7000`, recursive: false } }),
        "captured_desc",
        null,
        300,
      ),
    );
    // Il filtro cartella è tra i filtri attivi.
    const chip = screen.getByRole("button", {
      name: "Rimuovi filtro Cartella (solo questa): D7000",
    });

    // Scenario "Seleziona tutto in una cartella".
    commandsMock.librarySelection.mockResolvedValue({ count: 3, maxId: 6 });
    fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
    await waitFor(() => expect(selCount(useLibrary.getState().selection)).toBe(3));
    expect(useLibrary.getState().selection).toMatchObject({
      mode: "filter",
      filter: { folder: { path: `${trip}\\D7000`, recursive: false } },
    });
    expect(commandsMock.librarySelection).toHaveBeenLastCalledWith(
      expect.objectContaining({ folder: { path: `${trip}\\D7000`, recursive: false } }),
    );
    fireEvent.click(chip);
    expect(useLibrary.getState().filter.folder).toBeNull();
  });

  it("rinomina un luogo dal facet con conferma e conteggio", async () => {
    commandsMock.renamePlace.mockResolvedValue({ changed: 4, requested: 4 });
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: "Rinomina il luogo Siena" }));
    const dialog = screen.getByRole("dialog", { name: "Rinomina luogo" });
    expect(dialog).toHaveTextContent("4 foto");
    fireEvent.change(screen.getByLabelText("Nuovo nome"), { target: { value: "Toscana" } });
    fireEvent.click(screen.getByRole("button", { name: "Rinomina" }));
    await waitFor(() => expect(commandsMock.renamePlace).toHaveBeenCalledWith("Siena", "Toscana"));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("menu contestuale: selezione, Apri con e tastiera; doppio clic", async () => {
    renderPage();
    await screen.findByTestId("thumb-1");
    // Scenario "Clic destro su miniatura non selezionata".
    act(() => useLibrary.setState({ selection: idsSelection([1, 2, 3]), focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-5"));
    expect(selIds(useLibrary.getState().selection)).toEqual([5]);
    let menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    expect(menu).toHaveTextContent("1 foto");
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    // Scenario "Clic destro sulla selezione".
    act(() => useLibrary.setState({ selection: idsSelection([1, 2, 3]), focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-2"));
    menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    expect(menu).toHaveTextContent("3 foto");
    expect(selIds(useLibrary.getState().selection)).toEqual([1, 2, 3]);
    fireEvent.click(screen.getByRole("menuitem", { name: /^Apri con$/ }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Lightroom" }));
    await waitFor(() =>
      expect(commandsMock.openWith).toHaveBeenCalledWith(
        [1, 2, 3],
        { kind: "app", id: "lr" },
        false,
      ),
    );
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    // Scenario "Apertura da tastiera": Maiusc+F10, poi frecce.
    fireEvent.keyDown(screen.getByTestId("thumb-4"), { key: "F10", shiftKey: true });
    menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    const items = screen.getAllByRole("menuitem");
    await waitFor(() => expect(items[0]).toHaveFocus());
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(items[1]).toHaveFocus();
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(items[items.length - 1]).toHaveFocus();
    fireEvent.keyDown(menu, { key: "Escape" });
    fireEvent.keyDown(screen.getByTestId("thumb-4"), { key: "ContextMenu" });
    expect(screen.getByRole("menu", { name: "Azioni sulle foto" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("menuitem", { name: "Mostra nella cartella" }));
    await waitFor(() =>
      expect(commandsMock.openWith).toHaveBeenLastCalledWith([4], { kind: "reveal" }, false),
    );

    // Scenario "Doppio clic con applicazione configurata": il backend usa ui.double_click.
    fireEvent.doubleClick(screen.getByTestId("thumb-1"));
    await waitFor(() =>
      expect(commandsMock.openWith).toHaveBeenLastCalledWith([1], { kind: "double_click" }, false),
    );
  });

  it("imposta il luogo dal menu contestuale su tutta la selezione", async () => {
    renderPage();
    await screen.findByTestId("thumb-1");
    act(() => useLibrary.setState({ selection: idsSelection([1, 2]), focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Imposta luogo…" }));
    const menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    fireEvent.change(within(menu).getByLabelText("Luogo"), { target: { value: "Giappone" } });
    fireEvent.click(within(menu).getByRole("button", { name: "Imposta luogo" }));
    await waitFor(() =>
      expect(commandsMock.startBulk).toHaveBeenCalledWith(
        { kind: "ids", ids: [1, 2] },
        { action: "set_place", place: { name: "Giappone" } },
        false,
      ),
    );
  });
});

describe("gestione delle importazioni", () => {
  it("rimozione dalla Libreria: stessa conferma con riepilogo della pagina Import", async () => {
    commandsMock.sourceRemovalSummary.mockResolvedValue({
      items: 12,
      edited: 2,
      catalogs: [{ name: "Vacanze", items: 3 }],
      orphanedPeople: ["Giulia"],
      jobs: 1,
    } as never);
    commandsMock.removeSource.mockResolvedValue(null as never);
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /Azioni su/ }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rimuovi dalla libreria…" }));
    const dialog = await screen.findByRole("dialog", { name: /Rimuovere D:.Foto/ });
    expect(await within(dialog).findByText("12 foto verranno tolte dal catalogo.")).toBeTruthy();
    expect(within(dialog).getByText(/Vacanze \(3\)/)).toBeTruthy();
    expect(within(dialog).getByText(/Giulia/)).toBeTruthy();
    expect(within(dialog).getByText(/1 attività in corso/)).toBeTruthy();
    fireEvent.click(within(dialog).getByRole("button", { name: "Rimuovi dalla libreria" }));
    await waitFor(() => expect(commandsMock.removeSource).toHaveBeenCalledWith(1));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("le voci sui mancanti compaiono solo se la sorgente ne ha", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /Azioni su/ }));
    expect(screen.getByRole("menuitem", { name: "Ricollega mancanti…" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "Rimuovi mancanti…" })).toBeTruthy();
    fireEvent.click(screen.getByRole("menuitem", { name: "Aggiorna" }));
    await waitFor(() => expect(commandsMock.startImport).toHaveBeenCalledWith(1, null));
    cleanup();
    const [first] = (await commandsMock.listSources()) as unknown as SourceSummary[];
    commandsMock.listSources.mockResolvedValue([{ ...first, missing: 0 }] as never);
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /Azioni su/ }));
    expect(screen.getByRole("menuitem", { name: "Aggiorna cartella…" })).toBeTruthy();
    expect(screen.queryByRole("menuitem", { name: "Ricollega mancanti…" })).toBeNull();
    expect(screen.queryByRole("menuitem", { name: "Rimuovi mancanti…" })).toBeNull();
  });
});

describe("voci e gruppi creati dall'utente", () => {
  it("il gruppo solo manuale compare tra i filtri con i nomi delle voci", async () => {
    commandsMock.getTaxonomy.mockResolvedValue({
      version: 2,
      groups: [
        {
          id: "evento",
          label: "Evento",
          threshold: 0.25,
          categories: [{ id: "compleanno", label: "Compleanno", enabled: true, prompts: [] }],
        },
      ],
    } as never);
    commandsMock.libraryFacets.mockResolvedValue({
      labels: [["evento", list([{ value: "compleanno", count: 3 }])]],
      places: list([]),
      years: list([]),
      formats: list([]),
      people: list([]),
      tags: list([]),
    } as never);
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filtri" });
    expect(await within(filters).findByText("Evento")).toBeInTheDocument();
    fireEvent.click(await within(filters).findByRole("checkbox", { name: /Compleanno/ }));
    await waitFor(() =>
      expect(useLibrary.getState().filter.labels).toEqual([
        { group: "evento", values: ["compleanno"] },
      ]),
    );
    expect(await screen.findByRole("button", { name: "Rimuovi filtro Compleanno" })).toBeTruthy();
  });
});

describe("elenco completo dei filtri e pannelli flottanti", () => {
  const places = Array.from({ length: 340 }, (_, i) => ({ value: `Luogo ${i}`, count: 340 - i }));
  const subject = [
    { value: "ritratto", count: 9 },
    { value: "opera_arte", count: 5 },
    { value: "animale", count: 2 },
  ];
  beforeEach(() => {
    useFloating.setState({ panels: {}, order: [] });
    commandsMock.libraryFacets.mockResolvedValue({
      labels: [["subject", list(subject)]],
      places: list(places.slice(0, 50), 340),
      years: list([{ value: "2023", count: 12 }]),
      formats: list([]),
      people: list([]),
      tags: list([{ value: "mare", count: 3 }]),
    } as never);
    commandsMock.libraryFacetValues.mockImplementation((async (
      _f: unknown,
      key: { kind: string },
      search: string | null,
      sort: string,
      offset: number,
      limit: number,
    ) => {
      let values =
        key.kind === "place"
          ? places
          : key.kind === "label"
            ? subject
            : [{ value: "mare", count: 3 }];
      if (search)
        values = values.filter((v) => v.value.toLowerCase().includes(search.toLowerCase()));
      if (sort === "name") values = [...values].sort((a, b) => a.value.localeCompare(b.value));
      return { values: values.slice(offset, offset + limit), total: values.length };
    }) as never);
  });
  afterEach(() => i18n.changeLanguage("it"));

  it("molte località: Mostra tutti (340), ricerca e scelta del luogo con meno foto", async () => {
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filtri" });
    const showAll = await within(filters).findByRole("button", { name: "Mostra tutti (340)" });
    expect(within(filters).getByRole("checkbox", { name: /Luogo 7\b/ })).toBeTruthy();
    expect(within(filters).queryByRole("checkbox", { name: /Luogo 8\b/ })).toBeNull();
    fireEvent.click(showAll);
    const search = within(filters).getByRole("searchbox", { name: "Cerca in Luogo" });
    fireEvent.change(search, { target: { value: "339" } });
    await waitFor(() =>
      expect(commandsMock.libraryFacetValues).toHaveBeenCalledWith(
        expect.anything(),
        { kind: "place" },
        "339",
        "count",
        0,
        100,
      ),
    );
    fireEvent.click(await within(filters).findByRole("checkbox", { name: /Luogo 339/ }));
    await waitFor(() => expect(useLibrary.getState().filter.places).toEqual(["Luogo 339"]));
    fireEvent.click(within(filters).getByRole("button", { name: "Mostra meno" }));
    expect(within(filters).getByRole("button", { name: "Mostra tutti (340)" })).toBeTruthy();
  });

  it("la voce selezionata resta visibile e spuntata nella lista compatta", async () => {
    useLibrary.setState({ filter: { ...emptyFilter(), places: ["Luogo 339"] } });
    commandsMock.libraryFacets.mockResolvedValue({
      labels: [],
      places: list([...places.slice(0, 50), { value: "Luogo 339", count: 1 }], 340),
      years: list([]),
      formats: list([]),
      people: list([]),
      tags: list([]),
    } as never);
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filtri" });
    const chosen = await within(filters).findByRole("checkbox", { name: /Luogo 339/ });
    expect(chosen).toBeChecked();
  });

  it("pannello dei luoghi: selezione condivisa, aggiornamento col filtro, più pannelli", async () => {
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filtri" });
    fireEvent.click(await within(filters).findByRole("button", { name: "Apri Luogo in finestra" }));
    const panel = await screen.findByRole("dialog", { name: "Luogo" });
    expect(await within(panel).findByText("340 voci")).toBeTruthy();
    fireEvent.click(await within(panel).findByRole("checkbox", { name: /Luogo 5\b/ }));
    await waitFor(() => expect(useLibrary.getState().filter.places).toEqual(["Luogo 5"]));
    expect(within(filters).getByRole("checkbox", { name: /Luogo 5\b/ })).toBeChecked();
    // Il pannello segue gli altri filtri (anno dalla barra laterale).
    fireEvent.click(within(filters).getByRole("checkbox", { name: /2023/ }));
    await waitFor(() =>
      expect(commandsMock.libraryFacetValues).toHaveBeenCalledWith(
        expect.objectContaining({ dateFrom: "2023-01-01", places: ["Luogo 5"] }),
        { kind: "place" },
        null,
        "count",
        0,
        100,
      ),
    );
    // Un secondo pannello: i due restano aperti, l'ultimo cliccato in primo piano.
    fireEvent.click(within(filters).getByRole("button", { name: "Apri Tag in finestra" }));
    const tags = await screen.findByRole("dialog", { name: "Tag" });
    expect(screen.getByRole("dialog", { name: "Luogo" })).toBeTruthy();
    const z = (el: HTMLElement) => Number(el.style.zIndex);
    expect(z(tags)).toBeGreaterThan(z(panel));
    fireEvent.pointerDown(within(panel).getByText("340 voci"));
    expect(z(panel)).toBeGreaterThan(z(tags));
    fireEvent.click(within(panel).getByRole("button", { name: "Deseleziona tutto" }));
    await waitFor(() => expect(useLibrary.getState().filter.places).toEqual([]));
  });

  it("catalogazioni in ordine alfabetico sul nome tradotto", async () => {
    await i18n.changeLanguage("en");
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filters" });
    fireEvent.click(
      await within(filters).findByRole("button", { name: "Open Subject type in a window" }),
    );
    const panel = await screen.findByRole("dialog", { name: "Subject type" });
    fireEvent.change(within(panel).getByRole("combobox", { name: "Sort" }), {
      target: { value: "name" },
    });
    await waitFor(() => {
      const names = within(panel)
        .getAllByRole("checkbox")
        .map((c) => c.closest("label")?.textContent ?? "");
      expect(names.map((n) => n.replace(/\d+$/, ""))).toEqual([
        "Animal",
        "Portrait",
        "Work of art",
      ]);
    });
  });
});

describe("filtro per data anno → mese → giorno", () => {
  beforeEach(() => {
    commandsMock.libraryFacets.mockImplementation((async (f: {
      dateFrom: string | null;
      dateTo: string | null;
    }) => {
      const month = f.dateFrom?.startsWith("2023-05") && f.dateTo?.startsWith("2023-05");
      return {
        labels: [],
        places: list([]),
        years: list([
          { value: "2023", count: 7 },
          { value: "2022", count: 1 },
        ]),
        months: f.dateFrom?.startsWith("2023")
          ? list([
              { value: "2023-03", count: 4 },
              { value: "2023-05", count: 3 },
            ])
          : null,
        days: month
          ? list([
              { value: "2023-05-14", count: 2 },
              { value: "2023-05-20", count: 1 },
            ])
          : null,
        formats: list([]),
        people: list([]),
        tags: list([]),
      };
    }) as never);
  });
  afterEach(() => i18n.changeLanguage("it"));

  it("mesi sotto l'anno scelto, giorni sotto il mese, chip e risalita", async () => {
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filtri" });
    expect(within(filters).queryByRole("checkbox", { name: /maggio/ })).toBeNull();
    fireEvent.click(await within(filters).findByRole("checkbox", { name: /2023/ }));
    fireEvent.click(await within(filters).findByRole("checkbox", { name: /maggio/ }));
    await waitFor(() =>
      expect(commandsMock.libraryPage).toHaveBeenCalledWith(
        expect.objectContaining({ dateFrom: "2023-05-01", dateTo: "2023-05-31" }),
        expect.anything(),
        null,
        expect.any(Number),
      ),
    );
    expect(await screen.findByRole("button", { name: "Rimuovi filtro maggio 2023" })).toBeTruthy();
    fireEvent.click(await within(filters).findByRole("checkbox", { name: /dom 14/ }));
    await waitFor(() =>
      expect(useLibrary.getState().filter).toMatchObject({
        dateFrom: "2023-05-14",
        dateTo: "2023-05-14",
      }),
    );
    expect(
      await screen.findByRole("button", { name: "Rimuovi filtro 14 maggio 2023" }),
    ).toBeTruthy();
    // Togliere la spunta al giorno torna al mese.
    fireEvent.click(within(filters).getByRole("checkbox", { name: /dom 14/ }));
    await waitFor(() =>
      expect(useLibrary.getState().filter).toMatchObject({
        dateFrom: "2023-05-01",
        dateTo: "2023-05-31",
      }),
    );
    // Il chip toglie tutto il filtro per data.
    fireEvent.click(screen.getByRole("button", { name: "Rimuovi filtro maggio 2023" }));
    await waitFor(() => expect(useLibrary.getState().filter.dateFrom).toBeNull());
  });

  it("nomi di mesi e chip nella lingua dell'interfaccia", async () => {
    await i18n.changeLanguage("en");
    useLibrary.setState({
      filter: { ...emptyFilter(), dateFrom: "2023-05-14", dateTo: "2023-05-14" },
    });
    renderPage();
    const filters = await screen.findByRole("region", { name: "Filters" });
    expect(await within(filters).findByRole("checkbox", { name: /May/ })).toBeChecked();
    expect(within(filters).getByRole("checkbox", { name: /Sun 14/ })).toBeChecked();
    expect(await screen.findByRole("button", { name: /14 May 2023/ })).toBeTruthy();
  });
});

it("rivede le escluse smart, mostra tutte e reinserisce senza conferma", async () => {
  commandsMock.listCatalogs.mockResolvedValue([
    {
      catalog: { id: 5, name: "Viaggio", kind: "smart", rules: null, createdAt: 0 },
      items: 34,
      excluded: 6,
    },
  ] as never);
  commandsMock.startBulk.mockResolvedValue(799);
  useLibrary.setState({ filter: { ...emptyFilter(), catalogId: 5 }, selection: idsSelection() });
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: "Escluse (6)" }));
  expect(useLibrary.getState().filter).toMatchObject({
    catalogId: null,
    excludedFrom: 5,
    matchingOnly: true,
  });
  expect(useLibrary.getState().sort).toBe("excluded_at");
  expect(JSON.parse(localStorage.getItem("photo-cat.library")!).state.sort).toBe("captured_desc");
  fireEvent.click(screen.getByRole("checkbox", { name: /Mostra anche/ }));
  expect(useLibrary.getState().filter.matchingOnly).toBe(false);
  fireEvent.click(await screen.findByTestId("thumb-1"));
  fireEvent.keyDown(screen.getByTestId("grid-scroll"), { key: "Delete" });
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByTestId("grid-scroll"), { key: "Enter" });
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [1] },
      { action: "restore_to_catalog", catalog: 5 },
      false,
    ),
  );
  commandsMock.listCatalogs.mockResolvedValue([
    {
      catalog: { id: 5, name: "Viaggio", kind: "smart", rules: null, createdAt: 0 },
      items: 36,
      excluded: 4,
    },
  ] as never);
  act(() =>
    useLive.setState({
      jobs: { 799: { id: 799, state: "completed", phases: [], failed: 0 } as never },
    }),
  );
  expect((await screen.findByText(/foto reinserite/)).closest('[role="status"]')).not.toBeNull();
  // Il percorso riporta al catalogo.
  fireEvent.click(
    within(screen.getByRole("navigation", { name: "Percorso" })).getByRole("button", {
      name: "Viaggio",
    }),
  );
  expect(await screen.findByRole("button", { name: "Escluse (4)" })).toBeInTheDocument();
});

it("Canc fuori dal catalogo non fa nulla; dentro esclude una sola volta con Annulla", async () => {
  commandsMock.startBulk.mockResolvedValue(800);
  commandsMock.undoBulk.mockResolvedValue(null);
  const confirm = vi.spyOn(window, "confirm");
  useLibrary.setState({ selection: idsSelection([1]), filter: emptyFilter() });
  renderPage();
  fireEvent.keyDown(await screen.findByTestId("grid-scroll"), { key: "Delete" });
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
  act(() => useLibrary.getState().setFilter({ catalogId: 5 }));
  fireEvent.keyDown(screen.getByTestId("grid-scroll"), { key: "Delete" });
  fireEvent.keyDown(screen.getByTestId("grid-scroll"), { key: "Delete" });
  await waitFor(() => expect(commandsMock.startBulk).toHaveBeenCalledTimes(1));
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    { kind: "ids", ids: [1] },
    { action: "exclude_from_catalog", catalog: 5 },
    false,
  );
  expect(confirm).not.toHaveBeenCalled();
  act(() =>
    useLive.setState({
      jobs: { 800: { id: 800, state: "completed", phases: [], failed: 0 } as never },
    }),
  );
  expect(
    (await screen.findByText(/file non sono stati toccati/)).closest('[role="status"]'),
  ).not.toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Annulla" }));
  await waitFor(() => expect(commandsMock.undoBulk).toHaveBeenCalledTimes(1));
  confirm.mockRestore();
});

describe("contesto della vista", () => {
  it("Catalogo aperto: percorso, conteggio, Escluse e ritorno a Tutte le foto", async () => {
    commandsMock.libraryCount.mockResolvedValue(312 as never);
    commandsMock.listCatalogs.mockResolvedValue([
      {
        catalog: { id: 9, name: "Viaggio Toscana", kind: "manual", rules: null, createdAt: 0 },
        items: 312,
        excluded: 6,
      },
    ] as never);
    useLibrary.setState({ filter: { ...emptyFilter(), catalogId: 9 } });
    renderPage();
    const path = await screen.findByRole("navigation", { name: "Percorso" });
    await within(path).findByText("Viaggio Toscana");
    expect(path).toHaveTextContent("Tutte le foto›Viaggio Toscana");
    expect(await screen.findByTestId("library-header-count")).toHaveTextContent("312 foto");
    expect(screen.getByRole("button", { name: "Escluse (6)" })).toBeInTheDocument();
    fireEvent.click(within(path).getByRole("button", { name: "Tutte le foto" }));
    expect(useLibrary.getState().filter.catalogId).toBeNull();
  });

  it("Filtro delle foto simili: dal pannello Filtri al chip che lo rimuove", async () => {
    commandsMock.libraryCount.mockResolvedValue(3 as never);
    useLibrary.setState({ filter: emptyFilter() });
    renderPage();
    fireEvent.click(await screen.findByRole("checkbox", { name: "Solo foto con simili" }));
    expect(useLibrary.getState().filter.similarOnly).toBe(true);
    fireEvent.click(await screen.findByRole("button", { name: "Rimuovi filtro Con foto simili" }));
    expect(useLibrary.getState().filter.similarOnly).toBe(false);
  });
});
