import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { GridItem, PageAt } from "../../lib/ipc";
import { emptyFilter } from "../../store/library";
import { gridKey, MAX_PAGES, PAGE_SIZE, useGridPages, windowAction } from "./gridPages";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(cleanup);

const TOTAL = 100_000;
const item = (i: number): GridItem =>
  ({
    id: i + 1,
    name: `IMG_${i}.jpg`,
    path: "",
    thumb: null,
    capturedAt: null,
    width: null,
    height: null,
    status: "ok",
    faces: 0,
    analyzed: true,
    tags: [],
    excludedAt: null,
  }) as GridItem;

/** Libreria simulata: il cursore è la posizione dell'elemento. */
function fakeLibrary() {
  commandsMock.libraryPage.mockImplementation((async (
    _f: unknown,
    _s: unknown,
    at: PageAt | null,
    limit: number,
  ) => {
    let from = 0;
    if (at?.kind === "after") from = (at.cursor.key as number) + 1;
    if (at?.kind === "offset") from = at.offset;
    if (at?.kind === "before") from = Math.max(0, (at.cursor.key as number) - limit);
    const to = at?.kind === "before" ? (at.cursor.key as number) : Math.min(TOTAL, from + limit);
    const items = Array.from({ length: to - from }, (_, k) => item(from + k));
    const cursor = (i: number) => ({ key: i, id: i + 1 });
    return {
      items,
      next: to < TOTAL ? cursor(to - 1) : null,
      prev: from > 0 ? cursor(from) : null,
    };
  }) as never);
}

describe("griglia a memoria limitata", () => {
  it("dopo 50 pagine ne restano al massimo 5 e tornando su la pagina viene richiesta di nuovo", async () => {
    fakeLibrary();
    const qc = new QueryClient();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={qc}>{children}</QueryClientProvider>
    );
    const filter = emptyFilter();
    const { result } = renderHook(() => useGridPages(filter, "captured_desc", 0), { wrapper });
    await waitFor(() => expect(result.current.items.length).toBe(PAGE_SIZE));
    for (let n = 1; n < 50; n++) {
      await act(async () => {
        await result.current.pages.fetchNextPage();
      });
    }
    const cached = qc.getQueryData<{ pages: unknown[] }>(gridKey(filter, "captured_desc", 0));
    expect(cached?.pages.length).toBeLessThanOrEqual(MAX_PAGES);
    expect(result.current.items.length).toBeLessThanOrEqual(MAX_PAGES * PAGE_SIZE);
    // La finestra è sulle ultime pagine scorse, con le posizioni globali corrette.
    expect(result.current.start).toBe(45 * PAGE_SIZE);
    expect(result.current.items[0].id).toBe(45 * PAGE_SIZE + 1);
    expect(result.current.pages.hasPreviousPage).toBe(true);

    // Si torna su: la pagina scaricata viene richiesta di nuovo al backend (all'indietro).
    commandsMock.libraryPage.mockClear();
    await act(async () => {
      await result.current.pages.fetchPreviousPage();
    });
    expect(commandsMock.libraryPage).toHaveBeenCalledTimes(1);
    const at = commandsMock.libraryPage.mock.calls[0][2] as PageAt;
    expect(at.kind).toBe("before");
    expect(result.current.start).toBe(44 * PAGE_SIZE);
    expect(result.current.items[0].id).toBe(44 * PAGE_SIZE + 1);
    expect(
      qc.getQueryData<{ pages: unknown[] }>(gridKey(filter, "captured_desc", 0))?.pages.length,
    ).toBeLessThanOrEqual(MAX_PAGES);
  });

  it("un salto lontano riparte da un offset invece di scorrere pagina per pagina", async () => {
    fakeLibrary();
    const qc = new QueryClient();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={qc}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useGridPages(emptyFilter(), "captured_desc", 60_000), {
      wrapper,
    });
    await waitFor(() => expect(result.current.items.length).toBe(PAGE_SIZE));
    expect(result.current.start).toBe(60_000);
    expect(result.current.items[0].id).toBe(60_001);
    expect(commandsMock.libraryPage).toHaveBeenLastCalledWith(
      expect.anything(),
      "captured_desc",
      { kind: "offset", offset: 60_000 },
      PAGE_SIZE,
    );
  });

  it("decide cosa caricare in base alla vista", () => {
    const win = { start: 3000, len: 1500, hasNext: true, hasPrev: true };
    expect(windowAction({ from: 3500, to: 3600 }, win, 30)).toBeNull();
    expect(windowAction({ from: 4400, to: 4490 }, win, 30)).toBe("next");
    expect(windowAction({ from: 3010, to: 3100 }, win, 30)).toBe("prev");
    expect(windowAction({ from: 90_100, to: 90_200 }, win, 30)).toEqual({ seek: 90_000 });
  });
});
