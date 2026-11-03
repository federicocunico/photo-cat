import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { CatalogFeedback } from "./catalogActions";
import { useLive } from "../../store/live";
import { Inspector } from "./Inspector";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

beforeEach(() => {
  vi.clearAllMocks();
  useLibrary.setState({ focused: 7, selected: [7] });
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
afterEach(cleanup);

it("corregge il tipo soggetto e aggiunge tag e catalogo dall'ispettore", async () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  await screen.findByTestId("inspector");

  // Catalogazione: il valore corrente è preselezionato, lo si corregge in "Ritratto".
  const value = await screen.findByLabelText("Nuovo valore");
  await waitFor(() => expect(value).toHaveValue("paesaggio"));
  expect(screen.getByText("Correggi campo")).toBeDisabled();
  fireEvent.change(value, { target: { value: "ritratto" } });
  fireEvent.click(screen.getByText("Correggi campo"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      [7],
      { action: "set_label", group: "subject", value: "ritratto" },
      false,
    ),
  );

  // Tag utente.
  fireEvent.change(screen.getByLabelText("Nuovo tag"), { target: { value: " famiglia " } });
  fireEvent.click(screen.getByText("Aggiungi tag"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      [7],
      { action: "add_tag", name: "famiglia" },
      false,
    ),
  );

  // Catalogo: solo i cataloghi manuali sono proposti.
  const catalog = screen.getByLabelText("Catalogo");
  await screen.findByRole("option", { name: "Vacanze" });
  expect(screen.queryByRole("option", { name: "Smart" })).not.toBeInTheDocument();
  fireEvent.change(catalog, { target: { value: "3" } });
  fireEvent.click(screen.getByText("Aggiungi a catalogo"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      [7],
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
  const field = await screen.findByLabelText("Luogo");
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
      [7],
      {
        action: "set_place",
        place: { name: "Gifu", country: "JP", lat: 35.42, lon: 136.76 },
      },
      false,
    ),
  );
  // Scenario "Correzione del luogo": testo libero.
  fireEvent.change(field, { target: { value: "Giappone " } });
  fireEvent.click(screen.getByRole("button", { name: "Imposta luogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      [7],
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
  fireEvent.click(await screen.findByRole("button", { name: "Ripristina automatico" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      [7],
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
    expect(commandsMock.addCategory).toHaveBeenCalledWith("subject", "Barche", [], [7]),
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
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByRole("button", { name: "Escludi dal catalogo" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      [7],
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
