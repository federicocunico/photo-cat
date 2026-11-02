import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const uiPrefsLoad = vi.fn();
const uiPrefsSave = vi.fn<(p: unknown) => Promise<null>>(async () => null);
vi.mock("./ipc", () => ({
  isTauri: () => true,
  commands: { uiPrefsLoad: () => uiPrefsLoad(), uiPrefsSave: (p: unknown) => uiPrefsSave(p) },
}));

const { restoreUiPrefs, mirroredStorage, flushUiPrefs } = await import("./uiPrefs");

beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
});
afterEach(() => vi.useRealTimers());

describe("preferenze dell'interfaccia nel file dell'app", () => {
  it("profilo WebView2 cancellato: le preferenze tornano dal file", async () => {
    uiPrefsLoad.mockResolvedValue({
      "photo-cat.library": '{"state":{"sort":"name_asc"}}',
      "photo-cat.layout": '{"state":{"leftWidth":320}}',
    });
    await restoreUiPrefs();
    expect(localStorage.getItem("photo-cat.library")).toBe('{"state":{"sort":"name_asc"}}');
    expect(localStorage.getItem("photo-cat.layout")).toBe('{"state":{"leftWidth":320}}');
  });

  it("le preferenze già presenti vincono sul file, che viene aggiornato", async () => {
    localStorage.setItem("photo-cat.library", '{"state":{"sort":"size_desc"}}');
    uiPrefsLoad.mockResolvedValue({ "photo-cat.library": '{"state":{"sort":"name_asc"}}' });
    await restoreUiPrefs();
    expect(localStorage.getItem("photo-cat.library")).toBe('{"state":{"sort":"size_desc"}}');
    // Prima esecuzione dopo un aggiornamento: lo stato corrente va subito nel file.
    expect(uiPrefsSave).toHaveBeenCalledWith({
      "photo-cat.library": '{"state":{"sort":"size_desc"}}',
    });
  });

  it("file illeggibile: l'avvio prosegue senza errori", async () => {
    uiPrefsLoad.mockRejectedValue(new Error("io"));
    await expect(restoreUiPrefs()).resolves.toBeUndefined();
  });

  it("le modifiche vengono salvate nel file in un'unica scrittura ritardata", async () => {
    vi.useFakeTimers();
    mirroredStorage.setItem("photo-cat.layout", "a");
    mirroredStorage.setItem("photo-cat.layout", "b");
    mirroredStorage.setItem("photo-cat.floating", "c");
    expect(localStorage.getItem("photo-cat.layout")).toBe("b");
    expect(uiPrefsSave).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(600);
    expect(uiPrefsSave).toHaveBeenCalledTimes(1);
    expect(uiPrefsSave).toHaveBeenCalledWith({
      "photo-cat.layout": "b",
      "photo-cat.floating": "c",
    });
  });

  it("flush salva subito", async () => {
    mirroredStorage.setItem("photo-cat.library", "x");
    await flushUiPrefs();
    expect(uiPrefsSave).toHaveBeenCalledWith({ "photo-cat.library": "x" });
  });
});
