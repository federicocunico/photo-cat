import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import "../../test/layoutMock";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { GridItem } from "../../lib/ipc";
import { emptyFilter, idsSelection, selIds, useLibrary } from "../../store/library";
import { LibraryGrid } from "./Grid";
import { SimilarSection } from "./SimilarSection";
import { SelectionBar, SelectMenu } from "./SelectionBar";
import userEvent from "@testing-library/user-event";
import { displayTotal, locate, toBase } from "./similarView";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (k: string) => `thumb://${k}` }));

const item = (id: number, over: Partial<GridItem> = {}): GridItem => ({
  id,
  excludedAt: null,
  name: `IMG_${id}.JPG`,
  path: `D:\\Foto\\IMG_${id}.JPG`,
  thumb: null,
  capturedAt: null,
  width: 6000,
  height: 4000,
  status: "ok",
  faces: 0,
  analyzed: true,
  tags: [],
  similarGroup: null,
  similarCover: false,
  similarCount: null,
  ...over,
});

const wrap = (ui: React.ReactNode) =>
  render(<QueryClientProvider client={new QueryClient()}>{ui}</QueryClientProvider>);

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLibrary.setState({
    filter: emptyFilter(),
    selection: idsSelection(),
    focused: null,
    scrollTop: 0,
    thumbSize: 3,
    groupSimilar: true,
  });
});
afterEach(checkedCleanup(A11Y_KNOWN.library));

describe("posizioni nella griglia raggruppata", () => {
  it("le foto di un gruppo aperto seguono la rappresentante", () => {
    const exp = [{ group: 7, base: 1, members: [item(11), item(12)] }];
    expect(displayTotal(5, exp)).toBe(7);
    expect(locate(0, exp)).toEqual({ base: 0 });
    expect(locate(1, exp)).toEqual({ base: 1 });
    expect(locate(2, exp)).toMatchObject({ member: { id: 11 }, index: 0 });
    expect(locate(3, exp)).toMatchObject({ member: { id: 12 }, index: 1 });
    expect(locate(4, exp)).toEqual({ base: 2 });
    expect(toBase(3, exp)).toBe(1);
  });
});

describe("griglia raggruppata", () => {
  function setup() {
    // 8 foto del gruppo 7 rappresentate dalla principale (id 2), più due foto singole.
    commandsMock.libraryPage.mockResolvedValue({
      items: [item(1), item(2, { similarGroup: 7, similarCover: true, similarCount: 8 }), item(10)],
      next: null,
      prev: null,
    } as never);
    commandsMock.libraryCount.mockResolvedValue(10 as never);
    commandsMock.libraryGridCount.mockResolvedValue(3 as never);
    commandsMock.similarGroupMembers.mockResolvedValue({
      items: [2, 3, 4, 5, 6, 7, 8, 9].map((id) =>
        item(id, { similarGroup: 7, similarCover: id === 2 }),
      ),
      next: null,
      prev: null,
    } as never);
  }

  it('Gruppo chiuso: una sola miniatura con "8" e il conteggio sulle foto', async () => {
    setup();
    wrap(<LibraryGrid initialWidth={1200} />);
    const badge = await screen.findByTestId("group-badge-2");
    expect(badge).toHaveTextContent("8");
    expect(screen.queryByTestId("thumb-3")).not.toBeInTheDocument();
    expect(commandsMock.libraryPage).toHaveBeenCalledWith(
      expect.objectContaining({ groupSimilar: true }),
      "captured_desc",
      null,
      300,
    );
    // Il conteggio dei risultati resta sulle 10 foto.
    expect(screen.getByRole("region")).toHaveAttribute("aria-label", "10 elementi");
    // Selezionare il gruppo chiuso seleziona tutte le sue foto.
    fireEvent.click(screen.getByTestId("thumb-2"));
    await waitFor(() =>
      expect(selIds(useLibrary.getState().selection)).toEqual([2, 3, 4, 5, 6, 7, 8, 9]),
    );
  });

  it("Espansione: le 8 foto una accanto all'altra con la principale evidenziata", async () => {
    setup();
    wrap(<LibraryGrid initialWidth={1200} />);
    fireEvent.click(await screen.findByTestId("group-badge-2"));
    expect(await screen.findByTestId("thumb-9")).toBeInTheDocument();
    for (const id of [2, 3, 4, 5, 6, 7, 8, 9])
      expect(screen.getByTestId(`thumb-${id}`)).toBeInTheDocument();
    expect(screen.getByTestId("cover-2")).toHaveTextContent("Principale");
    // Ordine: dopo la principale, prima della foto singola successiva.
    const ids = screen
      .getAllByTestId(/^thumb-\d+$/)
      .map((e) => Number(e.getAttribute("data-testid")!.slice(6)));
    expect(ids).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    fireEvent.click(screen.getByTestId("group-badge-2"));
    await waitFor(() => expect(screen.queryByTestId("thumb-3")).not.toBeInTheDocument());
  });
});

describe("Inspector e scelta", () => {
  it("Inspector: le altre foto del gruppo, la principale e le correzioni", async () => {
    commandsMock.similarGroup.mockResolvedValue({
      id: 7,
      cover: 2,
      coverManual: false,
      members: [2, 3, 4].map((id) => ({ id, name: `IMG_${id}.JPG`, thumb: null })),
    } as never);
    wrap(<SimilarSection id={3} />);
    const section = await screen.findByTestId("similar-section");
    expect(section).toHaveTextContent("Foto simili (3)");
    expect(section).not.toHaveTextContent("Questa è la foto principale");
    fireEvent.click(screen.getByRole("button", { name: "Imposta come principale" }));
    await waitFor(() => expect(commandsMock.similarSetCover).toHaveBeenCalledWith(7, 3));
    fireEvent.click(screen.getByRole("button", { name: "Togli dal gruppo" }));
    await waitFor(() => expect(commandsMock.similarRemove).toHaveBeenCalledWith(3));
    cleanup();
    wrap(<SimilarSection id={2} />);
    expect(await screen.findByText("Questa è la foto principale del gruppo")).toBeInTheDocument();
  });

  it("Seleziona tutte tranne la principale: sui risultati o sulla selezione", async () => {
    commandsMock.similarNonCovers.mockResolvedValue([3, 4, 5] as never);
    const filter = { ...emptyFilter(), catalogId: 4, similarOnly: true };
    useLibrary.setState({ filter });
    const user = userEvent.setup();
    const pick = async (menu: string) => {
      screen.getByRole("button", { name: menu }).focus();
      await user.keyboard("{Enter}");
      await user.click(
        await screen.findByRole("menuitem", { name: /Seleziona tutte tranne la principale/ }),
      );
    };
    wrap(<SelectMenu />);
    await pick("Seleziona");
    await waitFor(() => expect(selIds(useLibrary.getState().selection)).toEqual([3, 4, 5]));
    expect(commandsMock.similarNonCovers).toHaveBeenLastCalledWith(filter);
    // Con una selezione esplicita si lavora su quella.
    cleanup();
    act(() => useLibrary.setState({ selection: idsSelection([2, 3, 9]) }));
    wrap(<SelectionBar />);
    await pick("Altro");
    await waitFor(() =>
      expect(commandsMock.similarNonCovers).toHaveBeenLastCalledWith({ ...filter, ids: [2, 3, 9] }),
    );
  });
});

describe("unione manuale", () => {
  it("Raggruppa come simili sulle foto selezionate", async () => {
    useLibrary.setState({ selection: idsSelection([4, 8, 15]) });
    const user = userEvent.setup();
    wrap(<SelectionBar />);
    screen.getByRole("button", { name: "Altro" }).focus();
    await user.keyboard("{Enter}");
    await user.click(await screen.findByRole("menuitem", { name: /Raggruppa come simili/ }));
    await waitFor(() => expect(commandsMock.similarGroupTogether).toHaveBeenCalledWith([4, 8, 15]));
  });
});

describe("Sequenza", () => {
  it("raggruppa gli elementi caricati: principale o prima foto, con il numero; aperto resta intero", async () => {
    const { collapseLoaded } = await import("./similarView");
    const it = (id: number, g: number | null, cover = false) => ({
      id,
      similarGroup: g,
      similarCover: cover,
    });
    const items = [it(1, null), it(2, 7), it(3, 7, true), it(4, 7), it(5, 9), it(6, 9)];
    expect(collapseLoaded(items, new Set()).map((s) => [s.item.id, s.count])).toEqual([
      [1, null],
      [3, 3],
      [5, 2],
    ]);
    expect(collapseLoaded(items, new Set([7])).map((s) => [s.item.id, s.count])).toEqual([
      [1, null],
      [2, null],
      [3, 3],
      [4, null],
      [5, 2],
    ]);
  });
});
