import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { commandsMock, emit, ipcMockModule } from "../test/ipcMock";
import { useLive, useLiveEvents } from "./live";

vi.mock("../lib/ipc", () => ipcMockModule(true));

afterEach(cleanup);

describe("conteggio errori dal backend", () => {
  it("all'avvio parte dal conteggio non archiviato e gli eventi live lo incrementano", async () => {
    useLive.setState({ errorCount: 0 });
    commandsMock.listJobs.mockResolvedValue([]);
    commandsMock.getResources.mockResolvedValue(null as never);
    commandsMock.errorCount.mockResolvedValue(12);
    renderHook(() => useLiveEvents());
    await waitFor(() => expect(useLive.getState().errorCount).toBe(12));
    act(() =>
      emit("errorBatch", {
        counts: [{ jobId: 1, phase: "scansione", code: "X", level: "ERROR", count: 3 }],
        recent: [],
      }),
    );
    expect(useLive.getState().errorCount).toBe(15);
  });
});
