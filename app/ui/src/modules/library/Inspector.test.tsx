import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { idsSelection, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useCatalogFeedback } from "./catalogActions";

function CatalogFeedback() {
  useCatalogFeedback();
  return null;
}
import { useLive } from "../../store/live";
import { ToastHost } from "../../components/feedback/ToastHost";
import { Inspector } from "./Inspector";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

beforeEach(() => {
  vi.clearAllMocks();
  useLibrary.setState({ focused: 7, selection: idsSelection([7]) });
  commandsMock.mediaDetail.mockResolvedValue({
    item: {
      id: 7,
      kind: "photo",
      sourceId: 1,
      path: "D:\\Foto\\IMG_0007.JPG",
      name: "IMG_0007.JPG",
      ext: "jpg",
      size: 1000,
      mtime: 0,
      quickHash: null,
      contentHash: null,
      addedAt: 0,
      updatedAt: 0,
      thumb: null,
      capturedAt: null,
      width: 6000,
      height: 4000,
      status: "ok",
      faces: 0,
      analyzed: true,
    },
    tech: null,
    dates: null,
    exif: null,
    location: null,
    labels: [{ group: "subject", value: "paesaggio", score: 0.82, origin: "auto" }],
    peopleCount: 0,
    people: [],
    faces: [],
    tags: ["tramonto"],
    catalogs: [],
    duplicates: [],
  } as never);
  commandsMock.getTaxonomy.mockResolvedValue({
    version: 1,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [
          { id: "paesaggio", label: "Paesaggio", enabled: true, prompts: [] },
          { id: "ritratto", label: "Ritratto", enabled: true, prompts: [] },
        ],
      },
    ],
  } as never);
  commandsMock.listCatalogs.mockResolvedValue([
    {
      catalog: { id: 3, name: "Vacanze", kind: "manual", rules: null, createdAt: 0 },
      items: 10,
    },
    {
      catalog: { id: 4, name: "Smart", kind: "smart", rules: null, createdAt: 0 },
      items: 2,
    },
  ] as never);
});
afterEach(checkedCleanup(A11Y_KNOWN.library));

it("corregge il tipo soggetto e aggiunge tag e catalogo dall'ispettore", async () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  await screen.findByTestId("inspector");

  // Scenario "Ispezione file": un clic sul valore del tipo soggetto lo rende modificabile.
  fireEvent.click(
    await screen.findByRole("button", { name: "Tipo soggetto: Paesaggio. Modifica" }),
  );
  const value = await screen.findByLabelText("Nuovo valore");
  await waitFor(() => expect(value).toHaveValue("paesaggio"));
  expect(screen.getByRole("button", { name: "Applica" })).toBeDisabled();
  fireEvent.change(value, { target: { value: "ritratto" } });
  fireEvent.click(screen.getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "set_label", group: "subject", value: "ritratto" },
      false,
    ),
  );

  // Tag utente: selettore con ricerca, Invio crea il tag.
  fireEvent.click(screen.getByRole("button", { name: /^Aggiungi un tag/ }));
  const tag = await screen.findByRole("combobox", { name: "Cerca o scrivi un tag" });
  fireEvent.change(tag, { target: { value: " famiglia " } });
  fireEvent.keyDown(tag, { key: "Enter" });
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "add_tag", name: "famiglia" },
      false,
    ),
  );

  // Catalogo: solo i cataloghi manuali sono proposti.
  fireEvent.click(screen.getByRole("button", { name: /^Aggiungi a un catalogo/ }));
  const vacanze = await screen.findByRole("option", { name: "Vacanze" });
  expect(screen.queryByRole("option", { name: "Smart" })).not.toBeInTheDocument();
  fireEvent.click(vacanze);
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "add_catalog", catalog: 3 },
      false,
    ),
  );
  expect(commandsMock.startBulk).toHaveBeenCalledTimes(3);
});

it("corregge il luogo con i suggerimenti e ripristina quello automatico", async () => {
  // Dettaglio preparato in beforeEach, con un luogo diverso.
  const base = (await (commandsMock.mediaDetail as unknown as () => Promise<object>)()) as Record<
    string,
    unknown
  >;
  commandsMock.mediaDetail.mockResolvedValue({
    ...base,
    location: { name: "D7000", country: null, lat: null, lon: null, source: "directory" },
  } as never);
  commandsMock.suggestPlaces.mockResolvedValue([
    { name: "Giappone", country: null, lat: null, lon: null, inLibrary: true },
    { name: "Gifu", country: "JP", lat: 35.42, lon: 136.76, inLibrary: false },
  ]);
  const { unmount } = render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Luogo: D7000. Modifica" }));
  const field = await screen.findByRole("textbox", { name: "Luogo" });
  await waitFor(() => expect(field).toHaveValue("D7000"));
  // Scenario "Suggerimenti del luogo".
  fireEvent.change(field, { target: { value: "Gi" } });
  await waitFor(() => expect(commandsMock.suggestPlaces).toHaveBeenCalledWith("Gi"));
  const gifu = await screen.findByRole("option", { name: /Gifu/ });
  expect(screen.getByRole("option", { name: /Giappone/ })).toHaveTextContent("in libreria");
  fireEvent.mouseDown(gifu);
  expect(field).toHaveValue("Gifu");
  fireEvent.click(screen.getByRole("button", { name: "Imposta luogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      {
        action: "set_place",
        place: { name: "Gifu", country: "JP", lat: 35.42, lon: 136.76 },
      },
      false,
    ),
  );
  // Scenario "Correzione del luogo": testo libero (la modifica sul campo si riapre).
  fireEvent.click(await screen.findByRole("button", { name: "Luogo: D7000. Modifica" }));
  fireEvent.change(await screen.findByRole("textbox", { name: "Luogo" }), {
    target: { value: "Giappone " },
  });
  fireEvent.click(screen.getByRole("button", { name: "Imposta luogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [7] },
      { action: "set_place", place: { name: "Giappone" } },
      false,
    ),
  );
  expect(screen.queryByRole("button", { name: "Ripristina automatico" })).not.toBeInTheDocument();
  unmount();

  // Con un luogo manuale compare "Ripristina automatico".
  commandsMock.mediaDetail.mockResolvedValue({
    ...base,
    location: { name: "Giappone", country: null, lat: null, lon: null, source: "manual" },
  } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Luogo: Giappone. Modifica" }));
  fireEvent.click(await screen.findByRole("button", { name: "Ripristina automatico" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [7] },
      { action: "set_place", place: null },
      false,
    ),
  );
});

it("nuova voce dall'Inspector: Invio crea e assegna, Esc annulla, nome obbligatorio", async () => {
  commandsMock.addCategory.mockResolvedValue({
    group: "subject",
    id: "barche",
    label: "Barche",
    created: true,
    job: 31,
  } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  fireEvent.click(
    await screen.findByRole("button", { name: "Tipo soggetto: Paesaggio. Modifica" }),
  );
  const value = await screen.findByLabelText("Nuovo valore");
  await screen.findByRole("option", { name: "Nuova voce…" });
  // Esc chiude il campo senza creare nulla.
  fireEvent.change(value, { target: { value: "__new__" } });
  fireEvent.keyDown(screen.getByLabelText("Nome della nuova voce"), { key: "Escape" });
  expect(screen.queryByLabelText("Nome della nuova voce")).not.toBeInTheDocument();
  // Nome vuoto: il campo lo segnala e nessuna voce viene creata.
  fireEvent.change(screen.getByLabelText("Nuovo valore"), { target: { value: "__new__" } });
  const name = screen.getByLabelText("Nome della nuova voce");
  fireEvent.keyDown(name, { key: "Enter" });
  expect(await screen.findByRole("alert")).toHaveTextContent("Il nome è obbligatorio");
  expect(commandsMock.addCategory).not.toHaveBeenCalled();
  // Invio con un nome: creata e assegnata alla foto.
  fireEvent.change(name, { target: { value: "Barche" } });
  fireEvent.keyDown(name, { key: "Enter" });
  await waitFor(() =>
    expect(commandsMock.addCategory).toHaveBeenCalledWith("subject", "Barche", [], {
      kind: "ids",
      ids: [7],
    }),
  );
  await waitFor(() =>
    expect(screen.queryByLabelText("Nome della nuova voce")).not.toBeInTheDocument(),
  );
});

it("chip volti: Apri in Persone imposta peopleTarget", async () => {
  const detail = await commandsMock.mediaDetail(7);
  commandsMock.mediaDetail.mockResolvedValue({
    ...detail,
    faces: [
      {
        id: 42,
        bbox: [0.1, 0.1, 0.2, 0.2],
        personId: 4,
        personName: "Giulia",
        identityUid: "g",
        ignored: false,
        discarded: false,
      },
      {
        id: 43,
        bbox: [0.5, 0.1, 0.2, 0.2],
        personId: null,
        personName: null,
        identityUid: null,
        ignored: false,
        discarded: false,
      },
      {
        id: 44,
        bbox: [0.7, 0.1, 0.2, 0.2],
        personId: 9,
        personName: null,
        identityUid: null,
        ignored: true,
        discarded: false,
      },
    ],
  } as never);
  useLayout.setState({ module: "library", peopleTarget: null });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  expect(await screen.findByTestId("inspector-face-42")).toHaveTextContent("Giulia");
  expect(screen.getByTestId("inspector-face-43")).toHaveTextContent(/volto non riconosciuto/i);
  expect(screen.getByTestId("inspector-face-44")).toHaveTextContent(/volto ignorato/i);
  const openButtons = screen.getAllByRole("button", { name: /Apri in Persone/i });
  fireEvent.click(openButtons[0]);
  expect(useLayout.getState().module).toBe("people");
  expect(useLayout.getState().peopleTarget).toEqual({ face: 42 });
});

it("esclude dal catalogo smart nell'Inspector", async () => {
  const detail = await commandsMock.mediaDetail(7);
  commandsMock.mediaDetail.mockResolvedValue({ ...detail, catalogs: ["Smart"] } as never);
  commandsMock.startBulk.mockResolvedValue(902);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
      <CatalogFeedback />
      <ToastHost />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Escludi dal catalogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [7] },
      { action: "exclude_from_catalog", catalog: 4 },
      false,
    ),
  );
  act(() =>
    useLive.setState({
      jobs: { 902: { id: 902, state: "completed", phases: [], failed: 0 } as never },
    }),
  );
  await screen.findByRole("status");
});

it("Selezione multipla: Valori diversi, poi il luogo vale per tutte", async () => {
  const detail = (await commandsMock.mediaDetail(7)) as unknown as Record<string, unknown>;
  commandsMock.mediaDetail.mockImplementation(
    async (id: number) =>
      ({
        ...detail,
        item: { ...(detail.item as object), id },
        location:
          id === 1
            ? { name: "Milano", country: null, lat: null, lon: null, source: "manual" }
            : { name: "Torino", country: null, lat: null, lon: null, source: "manual" },
        tags: id === 1 ? ["tramonto", "mare"] : ["tramonto"],
      }) as never,
  );
  useLibrary.setState({ focused: 2, selection: idsSelection([1, 2]) });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  expect(await screen.findByText("2 foto selezionate")).toBeInTheDocument();
  // Prima della modifica i luoghi diversi sono "Valori diversi"; i tag comuni restano.
  const place = await screen.findByRole("button", { name: "Luogo: Valori diversi. Modifica" });
  expect(await screen.findByText("tramonto")).toBeInTheDocument();
  expect(screen.queryByText("mare")).not.toBeInTheDocument();
  fireEvent.click(place);
  fireEvent.change(await screen.findByRole("textbox", { name: "Luogo" }), {
    target: { value: "Roma" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Imposta luogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [1, 2] },
      { action: "set_place", place: { name: "Roma" } },
      false,
    ),
  );
});

it("Sezione richiusa: resta richiusa anche su un'altra foto", async () => {
  useLayout.setState({ collapsedSections: [] });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  const tech = await screen.findByRole("button", { name: "Dati tecnici e percorso" });
  expect(screen.getByText("D:\\Foto\\IMG_0007.JPG")).toBeInTheDocument();
  fireEvent.click(tech);
  expect(screen.queryByText("D:\\Foto\\IMG_0007.JPG")).not.toBeInTheDocument();
  act(() => useLibrary.setState({ focused: 8, selection: idsSelection([8]) }));
  await screen.findByTestId("inspector");
  expect(screen.getByRole("button", { name: "Dati tecnici e percorso" })).toHaveAttribute(
    "aria-expanded",
    "false",
  );
});
