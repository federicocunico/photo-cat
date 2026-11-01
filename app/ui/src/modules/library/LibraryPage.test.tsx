import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import "../../test/layoutMock";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { clickSelect, emptyFilter, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { LibraryPage } from "./LibraryPage";
import { useFloating } from "../../store/floating";
import i18n from "../../i18n";
import type { GridItem, SourceSummary } from "../../bindings";

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
});

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLayout.setState({ leftOpen: true, rightOpen: true });
  useLibrary.setState({
    filter: emptyFilter(),
    selected: [],
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
      tags: ["tramonto"],
      catalogs: [],
      duplicates: [],
    })),
  });
});
afterEach(cleanup);

function renderPage() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <LibraryPage />
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
    expect(useLibrary.getState().selected).toEqual([1, 2, 3, 4]);
    fireEvent.click(screen.getByTestId("thumb-2"), { ctrlKey: true });
    expect(useLibrary.getState().selected).toEqual([1, 3, 4]);
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
    renderPage();
    await screen.findByTestId("thumb-1");
    fireEvent.change(screen.getByLabelText("Ordina"), { target: { value: "name_asc" } });
    await waitFor(() =>
      expect(commandsMock.libraryPage).toHaveBeenLastCalledWith(
        expect.anything(),
        "name_asc",
        null,
        300,
      ),
    );
    fireEvent.change(screen.getByLabelText("Dimensione miniature"), { target: { value: "5" } });
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
    commandsMock.libraryIds.mockResolvedValue([4, 5, 6]);
    fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
    await waitFor(() => expect(useLibrary.getState().selected).toEqual([4, 5, 6]));
    expect(commandsMock.libraryIds).toHaveBeenLastCalledWith(
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
    act(() => useLibrary.setState({ selected: [1, 2, 3], focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-5"));
    expect(useLibrary.getState().selected).toEqual([5]);
    let menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    expect(menu).toHaveTextContent("1 foto");
    fireEvent.keyDown(menu, { key: "Escape" });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    // Scenario "Clic destro sulla selezione".
    act(() => useLibrary.setState({ selected: [1, 2, 3], focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-2"));
    menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    expect(menu).toHaveTextContent("3 foto");
    expect(useLibrary.getState().selected).toEqual([1, 2, 3]);
    fireEvent.click(screen.getByRole("menuitem", { name: /Apri con ▸/ }));
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
    act(() => useLibrary.setState({ selected: [1, 2], focused: 1 }));
    fireEvent.contextMenu(screen.getByTestId("thumb-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Imposta luogo…" }));
    const menu = screen.getByRole("menu", { name: "Azioni sulle foto" });
    fireEvent.change(within(menu).getByLabelText("Luogo"), { target: { value: "Giappone" } });
    fireEvent.click(within(menu).getByRole("button", { name: "Imposta luogo" }));
    await waitFor(() =>
      expect(commandsMock.startBulk).toHaveBeenCalledWith(
        [1, 2],
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
    expect(within(dialog).getByText(/1 job in corso/)).toBeTruthy();
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
