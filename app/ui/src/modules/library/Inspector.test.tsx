import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLibrary } from "../../store/library";
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
