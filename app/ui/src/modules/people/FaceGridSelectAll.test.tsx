import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { FaceView } from "../../lib/ipc";
import { useGridShortcuts } from "../../commands/gridShortcuts";
import { useCommandStore } from "../../commands/registry";
import { FaceGrid } from "./FaceGrid";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const face = (id: number): FaceView => ({
  id,
  mediaId: id,
  bbox: [0.1, 0.1, 0.3, 0.3],
  quality: 0.9,
  qualityScore: 0.8,
  sharpness: 0.7,
  frontality: 0.9,
  sizePx: 64,
  discardReason: null,
  year: 2024,
  thumb: null,
  personId: 1,
});

const LOADED = Array.from({ length: 120 }, (_, i) => face(i + 1));
const ALL = Array.from({ length: 400 }, (_, i) => i + 1);

function Shell() {
  useGridShortcuts();
  return null;
}

function renderGrid() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <Shell />
      <div style={{ height: 600, width: 640, display: "flex" }}>
        <FaceGrid target={{ kind: "group", id: 1 }} group={null} />
      </div>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  commandsMock.listGroups.mockImplementation((async (section: string) =>
    section === "people"
      ? {
          items: [
            {
              id: 2,
              name: "Marco",
              identityUid: null,
              section: "people",
              faces: 5,
              goodFaces: 5,
              discardedFaces: 0,
              photos: 5,
              cover: null,
              ignoredAt: null,
              newFaces: 0,
              nameHint: null,
              mergeHint: null,
            },
          ],
          next: null,
          total: 1,
        }
      : { items: [], next: null, total: 0 }) as never);
  commandsMock.listIdentities.mockResolvedValue({
    identities: [],
    groups: [],
    storePath: "",
    defaultThreshold: 0.363,
  } as never);
  commandsMock.groupFaces.mockResolvedValue({
    items: LOADED,
    next: null,
    total: 400,
    goodCount: 400,
    discardedCount: 0,
  } as never);
  commandsMock.faceSelection.mockResolvedValue({ count: 400, maxId: 400 });
  commandsMock.faceSelectionIds.mockImplementation((async (
    _s: unknown,
    _m: number,
    except: number[],
  ) => ALL.filter((i) => !except.includes(i))) as never);
  commandsMock.correctPerson.mockResolvedValue(null);
});
afterEach(() => {
  cleanup();
  useCommandStore.setState({ commands: [] });
});

describe("FaceGrid Seleziona tutto", () => {
  it("Ctrl+A seleziona i 400 volti e Riassegna invia gli id risolti dal backend", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("400 selezionati");
    expect(commandsMock.faceSelection).toHaveBeenCalledWith({ kind: "group", group: 1 });

    const move = screen.getByLabelText(/Riassegna a/i) as HTMLSelectElement;
    await waitFor(() => expect(move.options.length).toBe(2));
    fireEvent.change(move, { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: /^Riassegna$/ }));
    await waitFor(() => expect(commandsMock.correctPerson).toHaveBeenCalled());
    expect(commandsMock.faceSelectionIds).toHaveBeenCalledWith(
      { kind: "group", group: 1 },
      400,
      [],
    );
    const arg = (commandsMock.correctPerson.mock.calls as unknown as [unknown][])[0]![0] as {
      action: string;
      faces: number[];
      to: number;
    };
    expect(arg.action).toBe("move");
    expect(arg.faces).toEqual(ALL);
  });

  it("Non è questa persona su «tutti» invia una sola correzione con i 400 id", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("400 selezionati");
    fireEvent.click(screen.getByRole("button", { name: /Non è questa persona/i }));
    await waitFor(() => expect(commandsMock.correctPerson).toHaveBeenCalled());
    expect(commandsMock.correctPerson).toHaveBeenCalledTimes(1);
    expect(commandsMock.correctPerson).toHaveBeenCalledWith({
      action: "reject",
      person: 1,
      faces: ALL,
    });
  });

  it("Ctrl+clic su un volto lo toglie dal «tutti» e Inverti scambia l'insieme", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("400 selezionati");
    fireEvent.click(screen.getByTestId("face-3"), { ctrlKey: true });
    await screen.findByText("399 selezionati");
    expect(screen.getByTestId("face-3").getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(screen.getByRole("button", { name: "Inverti selezione" }));
    await screen.findByText("1 selezionati");
    expect(screen.getByTestId("face-3").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByTestId("face-1").getAttribute("aria-pressed")).toBe("false");
  });

  it("Mostra nella cartella su «tutti» apre le foto di tutti i volti, anche non caricati", async () => {
    commandsMock.faceSelectionMedia.mockResolvedValue({
      present: [1, 2, 300],
      missing: ["D:/x/assente.jpg"],
    });
    commandsMock.openWith.mockResolvedValue({ opened: 3, needsConfirmation: false });
    renderGrid();
    await screen.findByTestId("face-1");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("400 selezionati");
    fireEvent.click(screen.getByTestId("face-5"), { ctrlKey: true });
    await screen.findByText("399 selezionati");
    fireEvent.contextMenu(screen.getByTestId("face-2"));
    fireEvent.click(await screen.findByRole("menuitem", { name: /Mostra nella cartella/ }));
    await waitFor(() => expect(commandsMock.openWith).toHaveBeenCalled());
    expect(commandsMock.faceSelectionMedia).toHaveBeenCalledWith(
      { kind: "group", group: 1 },
      400,
      [5],
    );
    expect(commandsMock.openWith).toHaveBeenCalledWith([1, 2, 300], { kind: "reveal" }, false);
    expect(await screen.findByText(/assente.jpg/)).toBeTruthy();
  });

  it("Esc svuota la selezione", async () => {
    renderGrid();
    await screen.findByTestId("face-1");
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("400 selezionati");
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText("400 selezionati")).toBeNull());
    expect(screen.getByTestId("face-1").getAttribute("aria-pressed")).toBe("false");
  });
});
