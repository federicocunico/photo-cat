import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import "../../test/layoutMock";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { clickSelect, emptyFilter, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { LibraryPage } from "./LibraryPage";
import type { GridItem } from "../../bindings";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

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
    getTaxonomy: vi.fn(async () => ({ version: 1, groups: [] })),
    libraryFacets: vi.fn(async () => ({
      labels: [
        [
          "subject",
          [
            { value: "paesaggio", count: 7 },
            { value: "ritratto", count: 5 },
          ],
        ],
      ],
      places: [{ value: "Siena", count: 4 }],
      years: [{ value: "2023", count: 12 }],
      formats: [{ value: "jpg", count: 12 }],
      people: [],
      tags: [],
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
    expect(insp).toHaveTextContent("Tipo soggetto: paesaggio");
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
    fireEvent.click(screen.getByRole("checkbox", { name: /paesaggio/ }));
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
    fireEvent.click(screen.getByRole("button", { name: "Rimuovi filtro paesaggio" }));
    await waitFor(() => expect(useLibrary.getState().filter.labels).toEqual([]));
    expect(useLibrary.getState().filter.places).toEqual(["Siena"]);

    // Sorgenti e cataloghi nel pannello sinistro.
    expect(screen.getByRole("button", { name: /D:\\Foto/ })).toHaveTextContent("12");
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
