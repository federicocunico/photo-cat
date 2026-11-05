import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { commandsMock, ipcMockModule } from "../../../test/ipcMock";
import type { Cell } from "../../../bindings";
import { useLayout } from "../../../store/layout";
import { useLibrary } from "../../../store/library";
import { AI_SECTIONS } from "../aiSections";
import { MapSection } from "./MapSection";

vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (k: string) => `thumb://${k}` }));
vi.mock("../../../lib/ipc", () => ipcMockModule(true));

const cm = commandsMock as unknown as Record<string, ReturnType<typeof vi.fn>>;
const BBOX = { south: 11.5, west: -5.25, north: 27.5, east: 5.5 };
const cellAt = (over: Partial<Cell> = {}): Cell => ({
  cx: 8,
  cy: 8,
  bbox: BBOX,
  manual: false,
  sampleThumbs: [],
  count: 12,
  approxCount: 2,
  lat: 20,
  lon: 0,
  place: "Roma",
  sampleIds: [1, 2],
  ...over,
});

function renderIt() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MapSection />
    </QueryClientProvider>,
  );
}

/** Clic sul centro della mappa (800x600 in jsdom): lon 0, lat 20. */
function clickCenter() {
  const map = screen.getByTestId("world-map");
  fireEvent.pointerDown(map, { pointerId: 1, button: 0, clientX: 400, clientY: 300 });
  fireEvent.pointerUp(map, { pointerId: 1, button: 0, clientX: 400, clientY: 300 });
}

class PointerEventPolyfill extends MouseEvent {
  pointerId: number;
  constructor(type: string, init: PointerEventInit = {}) {
    super(type, init);
    this.pointerId = init.pointerId ?? 0;
  }
}

beforeEach(() => {
  if (typeof window.PointerEvent === "undefined") vi.stubGlobal("PointerEvent", PointerEventPolyfill);
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  cm.mapCells = vi.fn(async () => [cellAt()]);
  cm.mapCatalogs = vi.fn(async () => []);
  cm.mapPlacesUnplaced = vi.fn(async () => []);
  cm.mapPlacesManual = vi.fn(async () => []);
  cm.mapPlaceSet = vi.fn(async () => null);
  cm.listCatalogs = vi.fn(async () => []);
  useLayout.setState({ module: "ai" });
  useLibrary.getState().resetFilter();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("sezione Mappa", () => {
  it("è tra le sezioni del modulo AI", () => {
    expect(AI_SECTIONS).toContain("map");
  });

  it("il clic su un gruppo apre la Libreria con l'area della cella", async () => {
    renderIt();
    await waitFor(() => expect(screen.getByText(/1 gruppi, 12 foto/)).toBeInTheDocument());
    clickCenter();
    expect(useLayout.getState().module).toBe("library");
    const g = useLibrary.getState().filter.geoBbox!;
    expect(g.label).toBe("Roma");
    expect(g.approx).toBe(true);
    expect(g).toEqual({ ...BBOX, approx: true, label: "Roma" });
  });

  it("il tooltip mostra le miniature, il conteggio e la posizione a mano", async () => {
    cm.mapCells = vi.fn(async () => [cellAt({ manual: true, sampleThumbs: ["a", null, "b", "c", "d", "e"] })]);
    renderIt();
    await waitFor(() => expect(screen.getByText(/1 gruppi, 12 foto/)).toBeInTheDocument());
    fireEvent.pointerMove(screen.getByTestId("world-map").parentElement!, { clientX: 400, clientY: 300 });
    const tip = await screen.findByRole("tooltip");
    const imgs = Array.from(tip.querySelectorAll("img")).map((i) => i.getAttribute("src"));
    expect(imgs).toEqual(["thumb://a", "thumb://b", "thumb://c", "thumb://d"]);
    expect(tip).toHaveTextContent("12 foto");
    expect(tip).toHaveTextContent("Posizione impostata a mano");
  });

  it("luoghi a mano: Rimuovi posizione chiama mapPlaceClear e ricarica", async () => {
    let list = [{ name: "Casa nonna", count: 5, lat: 1, lon: 2 }];
    cm.mapPlacesManual = vi.fn(async () => list);
    cm.mapPlaceClear = vi.fn(async () => {
      list = [];
      return null;
    });
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: "Rimuovi la posizione di Casa nonna" }));
    await waitFor(() => expect(cm.mapPlaceClear).toHaveBeenCalledWith("Casa nonna"));
    await waitFor(() => expect(cm.mapPlacesManual.mock.calls.length).toBeGreaterThan(1));
    await waitFor(() => expect(screen.queryByText("Casa nonna")).toBeNull());
  });

  it("luoghi a mano: Sposta usa il clic successivo per mapPlaceSet", async () => {
    cm.mapPlacesManual = vi.fn(async () => [{ name: "Casa nonna", count: 5, lat: 1, lon: 2 }]);
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: "Sposta Casa nonna" }));
    clickCenter();
    await waitFor(() => expect(cm.mapPlaceSet).toHaveBeenCalledTimes(1));
    const [name, lat, lon] = cm.mapPlaceSet.mock.calls[0];
    expect(name).toBe("Casa nonna");
    expect(lat).toBeCloseTo(20, 0);
    expect(lon).toBeCloseTo(0, 0);
  });

  it("l'interruttore delle posizioni approssimate rifà la richiesta con approx=false", async () => {
    renderIt();
    await waitFor(() => expect(cm.mapCells).toHaveBeenCalled());
    expect(cm.mapCells.mock.calls[0][3]).toBe(true);
    fireEvent.click(screen.getByLabelText("Mostra posizioni approssimate"));
    await waitFor(() => expect(cm.mapCells.mock.lastCall![3]).toBe(false));
  });

  it("il clic su un catalogo nell'area limita l'ambito", async () => {
    cm.mapCatalogs = vi.fn(async () => [{ catalogId: 3, name: "Viaggi", count: 4 }]);
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: /Viaggi/ }));
    await waitFor(() => expect(cm.mapCells.mock.lastCall![0].catalogId).toBe(3));
  });

  it("il selettore d'ambito esclude i cataloghi di sistema", async () => {
    const base = { kind: "manual", rules: null, createdAt: 0 };
    cm.listCatalogs = vi.fn(async () => [
      { catalog: { id: 1, name: "Cestino", system: "trash", ...base }, count: 0 },
      { catalog: { id: 2, name: "Viaggi", ...base }, count: 3 },
    ]);
    renderIt();
    expect(await screen.findByRole("option", { name: "Viaggi" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Cestino" })).toBeNull();
  });

  it("Posiziona: il clic successivo sulla mappa chiama mapPlaceSet e ricarica", async () => {
    cm.mapPlacesUnplaced = vi.fn(async () => [{ name: "Casa nonna", count: 5 }]);
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: "Posiziona Casa nonna" }));
    const before = cm.mapPlacesUnplaced.mock.calls.length;
    clickCenter();
    await waitFor(() => expect(cm.mapPlaceSet).toHaveBeenCalledTimes(1));
    const [name, lat, lon] = cm.mapPlaceSet.mock.calls[0];
    expect(name).toBe("Casa nonna");
    expect(lat).toBeCloseTo(20, 0);
    expect(lon).toBeCloseTo(0, 0);
    await waitFor(() => expect(cm.mapPlacesUnplaced.mock.calls.length).toBeGreaterThan(before));
    expect(useLayout.getState().module).toBe("ai");
  });
});

describe("rete", () => {
  it("non parte nessuna richiesta verso host esterni", async () => {
    const fetchMock = vi.fn().mockRejectedValue(new Error("offline"));
    vi.stubGlobal("fetch", fetchMock);
    renderIt();
    await waitFor(() => expect(cm.mapCells).toHaveBeenCalled());
    fireEvent.click(screen.getByLabelText("Ingrandisci"));
    fireEvent.keyDown(screen.getByTestId("world-map"), { key: "ArrowRight" });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 250));
    });
    expect(fetchMock).toHaveBeenCalled();
    for (const [input] of fetchMock.mock.calls) {
      const url = String(typeof input === "string" ? input : ((input as Request).url ?? input));
      expect(url).not.toMatch(/^(https?:)?\/\//i);
    }
  });
});

describe("accessibilità della mappa", () => {
  it("i gruppi dell'area sono elencati come pulsanti che aprono la Libreria", async () => {
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: "Apri Roma in Libreria (12 foto)" }));
    expect(useLayout.getState().module).toBe("library");
    expect(useLibrary.getState().filter.geoBbox).toEqual({ ...BBOX, approx: true, label: "Roma" });
  });

  it("Invio sulla mappa apre il gruppo al centro", async () => {
    renderIt();
    await waitFor(() => expect(screen.getByText(/1 gruppi, 12 foto/)).toBeInTheDocument());
    fireEvent.keyDown(screen.getByTestId("world-map"), { key: "Enter" });
    expect(useLayout.getState().module).toBe("library");
  });

  it("Posiziona da tastiera: Invio usa il centro della mappa", async () => {
    cm.mapPlacesUnplaced = vi.fn(async () => [{ name: "Casa nonna", count: 5 }]);
    renderIt();
    fireEvent.click(await screen.findByRole("button", { name: "Posiziona Casa nonna" }));
    fireEvent.keyDown(screen.getByTestId("world-map"), { key: "Enter" });
    await waitFor(() => expect(cm.mapPlaceSet).toHaveBeenCalledTimes(1));
    const [name, lat, lon] = cm.mapPlaceSet.mock.calls[0];
    expect(name).toBe("Casa nonna");
    expect(lat).toBeCloseTo(20, 0);
    expect(lon).toBeCloseTo(0, 0);
  });

  it("a zoom 1 i gruppi si ripetono sulle copie del mondo (antimeridiano)", async () => {
    renderIt();
    await waitFor(() => expect(screen.getByText(/1 gruppi, 12 foto/)).toBeInTheDocument());
    // mondo largo 256 px: la copia a est di Roma(0, 20) è a x = 400 + 256
    const map = screen.getByTestId("world-map");
    fireEvent.pointerDown(map, { pointerId: 1, button: 0, clientX: 656, clientY: 300 });
    fireEvent.pointerUp(map, { pointerId: 1, button: 0, clientX: 656, clientY: 300 });
    expect(useLayout.getState().module).toBe("library");
  });
});
