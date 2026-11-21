// Dal volto alla foto (task 4.1): menu contestuale, doppio clic e ritorno alla griglia.
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { FaceView, MediaDetail } from "../../lib/ipc";
import { FaceGrid } from "./FaceGrid";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const face = (id: number, mediaId: number): FaceView => ({
  id,
  mediaId,
  bbox: [0.1, 0.1, 0.3, 0.3],
  quality: 0.9,
  qualityScore: 0.8,
  sharpness: 0.7,
  frontality: 0.9,
  sizePx: 64,
  discardReason: null,
  year: 2024,
  thumb: `t${mediaId}`,
  personId: 1,
});

const detail = (id: number, status: "ok" | "missing" = "ok") =>
  ({
    item: { id, name: `foto${id}.jpg`, path: `D:/Foto/Mare/foto${id}.jpg`, status },
  }) as unknown as MediaDetail;

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
  commandsMock.listGroups.mockResolvedValue({ items: [], next: null, total: 0 } as never);
  commandsMock.listIdentities.mockResolvedValue({ identities: [] } as never);
  // 4 volti di 3 foto: i volti 1 e 2 sono nella stessa foto.
  commandsMock.groupFaces.mockResolvedValue({
    items: [face(1, 10), face(2, 10), face(3, 11), face(4, 12)],
    next: null,
    total: 4,
    goodCount: 4,
    discardedCount: 0,
  } as never);
  commandsMock.mediaDetail.mockImplementation(async (id: number) => detail(id));
  commandsMock.openWith.mockResolvedValue({ opened: 1, needsConfirmation: false });
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const menuItem = (name: RegExp) => screen.findByRole("menuitem", { name });

describe("FaceGrid: dal volto alla foto", () => {
  it("4 volti di 3 foto aprono 3 file", async () => {
    renderGrid();
    fireEvent.click(await screen.findByTestId("face-1"));
    fireEvent.click(screen.getByTestId("face-2"), { ctrlKey: true });
    fireEvent.click(screen.getByTestId("face-3"), { ctrlKey: true });
    fireEvent.click(screen.getByTestId("face-4"), { ctrlKey: true });
    fireEvent.contextMenu(screen.getByTestId("face-3"));
    fireEvent.click(await menuItem(/Mostra nella cartella/));
    await waitFor(() => expect(commandsMock.openWith).toHaveBeenCalled());
    expect(commandsMock.openWith).toHaveBeenCalledWith([10, 11, 12], { kind: "reveal" }, false);
  });

  it("un volto non selezionato agisce da solo", async () => {
    renderGrid();
    fireEvent.click(await screen.findByTestId("face-1"));
    fireEvent.contextMenu(screen.getByTestId("face-4"));
    fireEvent.click(await menuItem(/Mostra nella cartella/));
    await waitFor(() => expect(commandsMock.openWith).toHaveBeenCalled());
    expect(commandsMock.openWith).toHaveBeenCalledWith([12], { kind: "reveal" }, false);
  });

  it("foto mancante: avviso con il percorso e nessuna apertura", async () => {
    commandsMock.mediaDetail.mockImplementation(async (id: number) =>
      detail(id, id === 12 ? "missing" : "ok"),
    );
    renderGrid();
    fireEvent.contextMenu(await screen.findByTestId("face-4"));
    fireEvent.click(await menuItem(/Mostra nella cartella/));
    expect(await screen.findByText(/foto12\.jpg/)).toBeTruthy();
    expect(commandsMock.openWith).not.toHaveBeenCalled();
  });

  it("Mostra in Libreria naviga con la cartella e la foto del volto", async () => {
    renderGrid();
    fireEvent.contextMenu(await screen.findByTestId("face-3"));
    fireEvent.click(await menuItem(/Mostra in Libreria/));
    await waitFor(() => expect(useLayout.getState().module).toBe("library"));
    const lib = useLibrary.getState();
    expect(lib.filter.folder).toEqual({ path: "D:/Foto/Mare", recursive: false });
    expect(lib.focused).toBe(11);
  });

  it("doppio clic apre il visore col riquadro; alla chiusura la griglia resta com'era", async () => {
    renderGrid();
    fireEvent.click(await screen.findByTestId("face-2"));
    const scroll = screen.getByTestId("face-grid-scroll");
    scroll.scrollTop = 40;
    fireEvent.doubleClick(screen.getByTestId("face-3"));
    expect(await screen.findByTestId("viewer")).toBeTruthy();
    await act(async () => {
      fireEvent.keyDown(screen.getByTestId("viewer"), { key: "Escape" });
    });
    await waitFor(() => expect(screen.queryByTestId("viewer")).toBeNull());
    expect(screen.getByTestId("face-grid-scroll").scrollTop).toBe(40);
    expect(screen.getByTestId("face-2").getAttribute("aria-pressed")).toBe("true");
  });

  it("Invio su un volto apre il visore", async () => {
    renderGrid();
    fireEvent.keyDown(await screen.findByTestId("face-1"), { key: "Enter" });
    expect(await screen.findByTestId("viewer")).toBeTruthy();
  });
});

it("la cartella della foto si ricava anche dai percorsi di Windows", async () => {
  const { folderOf } = await import("./useFacePhotoActions");
  expect(folderOf("D:\\Foto\\Mare\\IMG_1.jpg")).toBe("D:\\Foto\\Mare");
  expect(folderOf("D:/Foto/Mare/IMG_1.jpg")).toBe("D:/Foto/Mare");
  expect(folderOf("\\\\nas\\foto\\1962\\scan.tif")).toBe("\\\\nas\\foto\\1962");
});
