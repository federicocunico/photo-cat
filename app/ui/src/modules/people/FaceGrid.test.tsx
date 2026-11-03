import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { FaceView, GroupRow } from "../../lib/ipc";
import { FaceGrid } from "./FaceGrid";
import { rectangleSelect } from "../../store/library";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { runUndo, useUndo } from "../../store/undo";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

function face(partial: Partial<FaceView> & { id: number }): FaceView {
  return {
    mediaId: 1,
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
    ...partial,
  };
}

function group(partial: Partial<GroupRow> & { id: number }): GroupRow {
  return {
    name: null,
    identityUid: null,
    section: "review",
    faces: 10,
    goodFaces: 9,
    discardedFaces: 1,
    photos: 8,
    cover: null,
    ignoredAt: null,
    newFaces: 0,
    nameHint: null,
    mergeHint: null,
    ...partial,
  };
}

function renderGrid(
  props: Partial<ComponentProps<typeof FaceGrid>> & {
    target: NonNullable<ComponentProps<typeof FaceGrid>["target"]>;
  },
) {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <div style={{ height: 600, width: 640, display: "flex" }}>
        <FaceGrid group={null} {...props} />
      </div>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listGroups.mockResolvedValue({ items: [], next: null, total: 0 } as never);
  commandsMock.groupFaces.mockResolvedValue({
    items: [
      face({ id: 1, year: 2023 }),
      face({ id: 2, year: 2022 }),
      face({ id: 3, year: 2021 }),
      face({
        id: 4,
        discardReason: "blurred",
        qualityScore: 0.1,
      }),
    ],
    next: null,
    total: 4,
    goodCount: 3,
    discardedCount: 1,
  } as never);
  commandsMock.correctPerson.mockResolvedValue(null);
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

describe("FaceGrid selezione e azioni", () => {
  it("seleziona con clic, Ctrl e Maiusc e invoca le azioni", async () => {
    commandsMock.listGroups.mockResolvedValue({
      items: [group({ id: 2, name: "Marco", faces: 5, section: "people" })],
      next: null,
      total: 1,
    } as never);

    renderGrid({
      target: { kind: "group", id: 1 },
      group: group({
        id: 1,
        faces: 214,
        photos: 168,
        goodFaces: 196,
        discardedFaces: 18,
        mergeHint: {
          otherId: 9,
          otherName: null,
          otherFaces: 17,
          otherPhotos: 15,
          otherCover: null,
          score: 0.58,
        },
      }),
    });

    await waitFor(() => expect(commandsMock.groupFaces).toHaveBeenCalled());
    expect(commandsMock.groupFaces.mock.calls[0][0]).toEqual({ kind: "group", group: 1 });

    const f1 = await screen.findByTestId("face-1");
    fireEvent.click(f1);
    expect(f1).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText(/1 selezionat/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("face-3"), { ctrlKey: true });
    expect(screen.getByTestId("face-1")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("face-3")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText(/2 selezionat/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("face-1"));
    fireEvent.click(screen.getByTestId("face-3"), { shiftKey: true });
    expect(screen.getByTestId("face-1")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("face-2")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("face-3")).toHaveAttribute("aria-pressed", "true");

    const move = screen.getByLabelText(/Sposta in/i) as HTMLSelectElement;
    await waitFor(() =>
      expect([...move.options].some((o) => o.textContent?.includes("Marco"))).toBe(true),
    );
    fireEvent.change(move, { target: { value: "2" } });
    expect(commandsMock.correctPerson).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /^Sposta$/ }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "move",
        faces: [1, 2, 3],
        to: 2,
      }),
    );

    fireEvent.click(await screen.findByTestId("face-1"));
    fireEvent.click(screen.getByTestId("face-2"), { ctrlKey: true });
    fireEvent.click(screen.getByRole("button", { name: /Non è questa persona/i }));
    await waitFor(() => {
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "reject",
        person: 1,
        face: 1,
      });
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "reject",
        person: 1,
        face: 2,
      });
    });

    fireEvent.click(await screen.findByTestId("face-1"));
    fireEvent.click(screen.getByRole("button", { name: /Nuovo gruppo/i }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "separate",
        person: 1,
        faces: [1],
      }),
    );

    // Rimuovi volto non si annulla: chiede conferma con il verbo esplicito.
    fireEvent.click(await screen.findByTestId("face-1"));
    fireEvent.click(screen.getByRole("button", { name: /Rimuovi volto/i }));
    const dialog = await screen.findByRole("alertdialog", { name: /Rimuovere il volto/i });
    expect(commandsMock.correctPerson).not.toHaveBeenCalledWith({ action: "remove", face: 1 });
    fireEvent.click(within(dialog).getByRole("button", { name: /^Rimuovi volto$/i }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({ action: "remove", face: 1 }),
    );
  });

  it("«Sposta in…»: cambiare la scelta da tastiera non sposta nulla finché non si conferma", async () => {
    commandsMock.listGroups.mockResolvedValue({
      items: [
        group({ id: 2, name: "Marco", faces: 5, section: "people" }),
        group({ id: 3, name: "Anna", faces: 7, section: "people" }),
      ],
      next: null,
      total: 2,
    } as never);
    commandsMock.undoPeople.mockResolvedValue(null);
    renderGrid({ target: { kind: "group", id: 1 }, group: group({ id: 1 }) });

    fireEvent.click(await screen.findByTestId("face-1"));
    const move = screen.getByLabelText(/Sposta in/i) as HTMLSelectElement;
    await waitFor(() => expect(move.options.length).toBe(3));
    const apply = screen.getByRole("button", { name: /^Sposta$/ });
    // Senza destinazione il comando spiega perché non è disponibile.
    expect(apply).toHaveAttribute("aria-disabled", "true");

    // Le frecce su una select chiusa (Windows) cambiano il valore: nessuno spostamento.
    move.focus();
    fireEvent.keyDown(move, { key: "ArrowDown" });
    fireEvent.change(move, { target: { value: "2" } });
    fireEvent.keyDown(move, { key: "ArrowDown" });
    fireEvent.change(move, { target: { value: "3" } });
    await new Promise((r) => setTimeout(r, 20));
    expect(commandsMock.correctPerson).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /^Sposta$/ }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "move",
        faces: [1],
        to: 3,
      }),
    );
    expect(commandsMock.correctPerson).toHaveBeenCalledTimes(1);

    // Lo spostamento è annullabile dallo store `undo`.
    await waitFor(() => expect(useUndo.getState().last?.label).toMatch(/Volto spostato in «Anna»/));
    await runUndo();
    expect(commandsMock.undoPeople).toHaveBeenCalledTimes(1);
  });

  it("filtra i scartati e mostra il motivo in tooltip; banner unione", async () => {
    renderGrid({
      target: { kind: "group", id: 1 },
      group: group({
        id: 1,
        faces: 20,
        mergeHint: {
          otherId: 9,
          otherName: null,
          otherFaces: 17,
          otherPhotos: 15,
          otherCover: null,
          score: 0.58,
        },
      }),
    });

    expect(await screen.findByText(/Somiglia a/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Sono la stessa persona/i }));
    expect(await screen.findByRole("dialog", { name: /Unisci gruppi/i })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Unisci a/i }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "merge",
        from: 9,
        to: 1,
      }),
    );

    commandsMock.groupFaces.mockResolvedValue({
      items: [
        face({
          id: 40,
          discardReason: "too_small",
          sizePx: 28,
        }),
      ],
      next: null,
      total: 1,
      goodCount: 19,
      discardedCount: 1,
    } as never);

    fireEvent.click(screen.getByRole("button", { name: /Solo scartati per qualità/i }));
    await waitFor(() =>
      expect(commandsMock.groupFaces).toHaveBeenCalledWith(
        { kind: "discarded", group: 1 },
        "quality_desc",
        null,
        80,
      ),
    );
    const discarded = await screen.findByTestId("face-40");
    expect(discarded).toHaveAttribute("title", expect.stringMatching(/troppo piccolo.*28/i));
  });

  it("rettangolo di selezione indipendente dai nodi virtualizzati", () => {
    const ids = Array.from({ length: 100_000 }, (_, i) => i + 1);
    expect(
      rectangleSelect(
        ids,
        { x: 12, y: 12 + 128, w: 200, h: 80 },
        { cols: 4, cell: 96, rowHeight: 128, gap: 10, pad: 12 },
      ),
    ).toEqual([5, 6]);
  });
});

describe("Libreria grande", () => {
  it("con 100.000 volti simulati tiene limitato il numero di nodi DOM", async () => {
    const TOTAL = 100_000;
    const items = Array.from({ length: TOTAL }, (_, i) =>
      face({
        id: i + 1,
        qualityScore: 1 - i / TOTAL,
        year: 2000 + (i % 25),
        thumb: null,
      }),
    );
    commandsMock.groupFaces.mockResolvedValue({
      items,
      next: null,
      total: TOTAL,
      goodCount: TOTAL - 10,
      discardedCount: 10,
    } as never);

    renderGrid({
      target: { kind: "group", id: 1 },
      group: group({ id: 1, faces: TOTAL, photos: 50_000 }),
    });

    await waitFor(() => expect(screen.getByTestId("face-grid-scroll")).toBeInTheDocument());
    await waitFor(() => expect(screen.queryAllByTestId(/^face-/).length).toBeGreaterThan(0));

    const nodes = screen.queryAllByTestId(/^face-/);
    expect(nodes.length).toBeGreaterThan(0);
    expect(nodes.length).toBeLessThan(500);
    expect(commandsMock.groupFaces).toHaveBeenCalled();
  });
});
