import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { commandsMock, ipcMockModule } from "../test/ipcMock";
import { latestOnly, Superseded, useLibraryCount } from "./latest";
import type { LibraryFilter, RequestTag } from "./ipc";
import { emptyFilter } from "../store/library";

vi.mock("./ipc", () => ipcMockModule(true));
afterEach(cleanup);

function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe("richieste superate", () => {
  it("una risposta arrivata dopo una richiesta più recente viene scartata", async () => {
    const a = deferred<number | null>();
    const b = deferred<number | null>();
    const first = latestOnly("t", () => a.promise);
    const second = latestOnly("t", () => b.promise);
    b.resolve(2);
    a.resolve(1);
    await expect(second).resolves.toBe(2);
    await expect(first).rejects.toBeInstanceOf(Superseded);
  });

  it("canali diversi non si influenzano e null dal backend vale come superata", async () => {
    await expect(latestOnly("x", async () => 1)).resolves.toBe(1);
    await expect(latestOnly("y", async () => 2)).resolves.toBe(2);
    await expect(latestOnly("x", async () => null)).rejects.toBeInstanceOf(Superseded);
  });

  it("griglia: con risposte fuori ordine il conteggio mostrato è quello dell'ultimo filtro", async () => {
    const pending = new Map<string, ReturnType<typeof deferred<number | null>>>();
    const tags: RequestTag[] = [];
    commandsMock.libraryCount.mockImplementation(((f: LibraryFilter, tag: RequestTag) => {
      tags.push(tag);
      const d = deferred<number | null>();
      pending.set(f.text ?? "", d);
      return d.promise;
    }) as never);
    let setText: (t: string) => void = () => {};
    function Count() {
      const [text, set] = useState("a");
      setText = set;
      const q = useLibraryCount({ ...emptyFilter(), text });
      return <p data-testid="n">{q.data ?? "…"}</p>;
    }
    render(
      <QueryClientProvider client={new QueryClient()}>
        <Count />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(pending.has("a")).toBe(true));
    act(() => setText("b"));
    await waitFor(() => expect(pending.has("b")).toBe(true));
    expect(tags.map((t) => t.channel)).toEqual(["library-count", "library-count"]);
    expect(tags[1].id).toBeGreaterThan(tags[0].id);
    // Prima risponde il filtro nuovo, poi quello vecchio (in ritardo).
    await act(async () => pending.get("b")!.resolve(7));
    await act(async () => pending.get("a")!.resolve(99));
    await waitFor(() => expect(screen.getByTestId("n")).toHaveTextContent("7"));
    expect(commandsMock.libraryCount).toHaveBeenCalledTimes(2);
  });
});
