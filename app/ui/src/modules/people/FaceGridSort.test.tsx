import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { FaceView } from "../../lib/ipc";
import { FaceGrid } from "./FaceGrid";
import { rectangleSelectRows, buildFaceRows } from "./faceLayout";
import { usePeoplePrefs } from "../../store/peoplePrefs";
import { cleanup } from "@testing-library/react";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const face = (id: number, year: number | null): FaceView => ({
  id,
  mediaId: id,
  bbox: [0.1, 0.1, 0.3, 0.3],
  quality: 0.9,
  qualityScore: 0.8,
  sharpness: 0.7,
  frontality: 0.9,
  sizePx: 64,
  discardReason: null,
  year,
  thumb: null,
  personId: 1,
});

const items = [face(1, 2021), face(2, 2021), face(3, 2020), face(4, null)];

function renderGrid() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <div style={{ height: 600, width: 640, display: "flex" }}>
        <FaceGrid target={{ kind: "group", id: 1 }} group={null} />
      </div>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  usePeoplePrefs.setState({ faceSort: "quality_desc" });
  commandsMock.listGroups.mockResolvedValue({ items: [], next: null, total: 0 } as never);
  commandsMock.groupFaces.mockResolvedValue({
    items,
    next: null,
    total: 4,
    goodCount: 4,
    discardedCount: 0,
  } as never);
  commandsMock.groupFaceYears.mockResolvedValue([
    { year: null, count: 1 },
    { year: 2020, count: 1 },
    { year: 2021, count: 2 },
  ]);
});
afterEach(cleanup);

const lastSort = () => {
  const c = commandsMock.groupFaces.mock.calls;
  return c[c.length - 1]?.[1];
};
const pick = (v: string) =>
  fireEvent.change(screen.getByLabelText(/Ordina per/), { target: { value: v } });

describe("ordinamento dei volti per data", () => {
  it("il cambio di ordinamento invia il sort giusto", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    expect(commandsMock.groupFaces.mock.calls[0][1]).toBe("quality_desc");
    pick("captured_asc");
    await waitFor(() =>
      expect(commandsMock.groupFaces).toHaveBeenLastCalledWith(
        { kind: "group", group: 1 },
        "captured_asc",
        null,
        expect.any(Number),
      ),
    );
    pick("captured_desc");
    await waitFor(() => expect(lastSort()).toBe("captured_desc"));
  });

  it("mostra intestazioni per anno con conteggi e Senza data in fondo", async () => {
    usePeoplePrefs.setState({ faceSort: "captured_desc" });
    renderGrid();
    expect(await screen.findByText("2021 · 2 volti")).toBeInTheDocument();
    expect(screen.getByText("2020 · 1 volto")).toBeInTheDocument();
    const heads = screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(heads).toEqual(["2021 · 2 volti", "2020 · 1 volto", "Senza data · 1 volto"]);
    expect(commandsMock.groupFaceYears).toHaveBeenCalledWith({ kind: "group", group: 1 });
  });

  it("dal più vecchio ordina gli anni in modo crescente", async () => {
    usePeoplePrefs.setState({ faceSort: "captured_asc" });
    renderGrid();
    await screen.findByText("2020 · 1 volto");
    const heads = screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(heads[0]).toBe("2020 · 1 volto");
    expect(heads[heads.length - 1]).toBe("Senza data · 1 volto");
  });

  it("la scelta è ricordata nelle preferenze", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    pick("captured_asc");
    await waitFor(() =>
      expect(JSON.parse(localStorage.getItem("photo-cat.people") ?? "{}").state.faceSort).toBe(
        "captured_asc",
      ),
    );
    cleanup();
    renderGrid();
    await waitFor(() => expect(lastSort()).toBe("captured_asc"));
    expect(await screen.findByLabelText(/Ordina per/)).toHaveValue("captured_asc");
  });

  it("la selezione resta al cambio di ordinamento", async () => {
    renderGrid();
    fireEvent.click(await screen.findByTestId("face-2"));
    pick("captured_desc");
    await waitFor(() => expect(lastSort()).toBe("captured_desc"));
    await screen.findByText("2021 · 2 volti");
    expect(screen.getByTestId("face-2")).toHaveAttribute("aria-pressed", "true");
  });
});

describe("selezione rettangolare con intestazioni", () => {
  it("salta le intestazioni", () => {
    const rows = buildFaceRows(
      [
        { year: 2021, count: 2 },
        { year: 2020, count: 1 },
      ],
      4,
    );
    const g = { cell: 100, rowHeight: 130, headHeight: 32, gap: 10, pad: 12 };
    // Riga 1 volti: y 44..174; riga 2 volti: y 206..336.
    expect(rectangleSelectRows([1, 2, 3], { x: 0, y: 0, w: 1000, h: 1000 }, rows, g)).toEqual([
      1, 2, 3,
    ]);
    expect(rectangleSelectRows([1, 2, 3], { x: 0, y: 200, w: 1000, h: 50 }, rows, g)).toEqual([3]);
  });
});
