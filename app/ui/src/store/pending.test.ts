import { afterEach, expect, it, vi } from "vitest";
import { trackCommand, usePending } from "./pending";
import { useToast } from "./toasts";

afterEach(() => {
  vi.useRealTimers();
  usePending.setState({ count: 0 });
  useToast.setState({ notices: [] });
});
it("counts concurrent commands, excludes polling and cleans up failures", async () => {
  let finish!: () => void;
  const pending = trackCommand(
    "listSources",
    () =>
      new Promise<void>((r) => {
        finish = r;
      }),
  );
  expect(usePending.getState().count).toBe(1);
  await trackCommand("listJobs", async () => {
    expect(usePending.getState().count).toBe(1);
  });
  await expect(
    trackCommand("saveIdentity", async () => {
      throw Error("failed");
    }),
  ).rejects.toThrow("failed");
  expect(usePending.getState().count).toBe(1);
  finish();
  await pending;
  expect(usePending.getState().count).toBe(0);
});
it("announces the outcome of a slow save only after completion", async () => {
  vi.useFakeTimers();
  const saving = trackCommand("saveIdentity", () => new Promise((r) => setTimeout(r, 1200)));
  expect(useToast.getState().notices).toHaveLength(0);
  await vi.advanceTimersByTimeAsync(1200);
  await saving;
  expect(useToast.getState().notices[0].message).toBe("Salvataggio completato");
});
