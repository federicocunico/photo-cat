import { afterEach, expect, it, vi } from "vitest";
import i18n from "../i18n";
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
it("E6: un salvataggio lento non mostra avvisi generici, un errore lento sì", async () => {
  vi.useFakeTimers();
  const saving = trackCommand("saveIdentity", () => new Promise((r) => setTimeout(r, 1200)));
  await vi.advanceTimersByTimeAsync(1200);
  await saving;
  expect(useToast.getState().notices).toHaveLength(0);
  const failing = trackCommand(
    "saveIdentity",
    () => new Promise((_, reject) => setTimeout(() => reject(Error("no")), 1200)),
  );
  const caught = failing.catch(() => undefined);
  await vi.advanceTimersByTimeAsync(1200);
  await caught;
  expect(useToast.getState().notices[0].message).toBe("Operazione non riuscita");
});

it("E6: i testi generici «Operazione completata» e «Salvataggio completato» non esistono più", () => {
  for (const key of ["feedback.done", "feedback.saved", "feedback.merged"])
    expect(i18n.exists(key)).toBe(false);
});
