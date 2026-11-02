import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { act, cleanup, render, renderHook } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { emit, ipcMockModule, job } from "../test/ipcMock";
import { useLive, useLiveEvents, useTrimCaches } from "./live";

vi.mock("../lib/ipc", () => ipcMockModule());

beforeEach(() => {
  useLive.setState({ jobs: {}, resources: null, errorCount: 0, lastError: null, tail: [] });
});
afterEach(cleanup);

describe("aggiornamenti dei job", () => {
  it("upsertJob sostituisce solo il job cambiato e ignora gli snapshot identici", () => {
    renderHook(() => useLiveEvents());
    act(() => {
      emit("job", job({ id: 1, overallPercent: 10 }));
      emit("job", job({ id: 2, overallPercent: 50 }));
    });
    const before = useLive.getState().jobs;
    const two = before[2];
    act(() => emit("job", job({ id: 1, overallPercent: 10 })));
    expect(useLive.getState().jobs).toBe(before);
    act(() => emit("job", job({ id: 1, overallPercent: 11 })));
    const after = useLive.getState().jobs;
    expect(after).not.toBe(before);
    expect(after[1].overallPercent).toBe(11);
    expect(after[2]).toBe(two);
  });
});

describe("errori a lotti", () => {
  it("un lotto aggiorna il contatore una volta con la somma degli ERROR", () => {
    renderHook(() => useLiveEvents());
    const updates = vi.fn();
    const off = useLive.subscribe(updates);
    act(() =>
      emit("errorBatch", {
        counts: [
          { jobId: 1, phase: "miniature", code: "DECODE_FAILED", level: "ERROR", count: 1240 },
          { jobId: 1, phase: "metadati", code: "CORRUPT_EXIF", level: "WARN", count: 30 },
        ],
        recent: [],
      }),
    );
    off();
    expect(updates).toHaveBeenCalledTimes(1);
    expect(useLive.getState().errorCount).toBe(1240);
  });
});

describe("app://trim-caches", () => {
  it("elimina le query non mostrate e conserva quelle attive", async () => {
    const qc = new QueryClient();
    qc.setQueryData(["inattiva"], [1, 2, 3]);
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client: qc }, children);
    function Active() {
      useTrimCaches();
      useQuery({ queryKey: ["attiva"], queryFn: async () => "x" });
      return null;
    }
    render(createElement(Active), { wrapper });
    await vi.waitFor(() => expect(qc.getQueryData(["attiva"])).toBe("x"));
    act(() => emit("trimCaches", undefined));
    expect(qc.getQueryData(["inattiva"])).toBeUndefined();
    expect(qc.getQueryData(["attiva"])).toBe("x");
  });
});
