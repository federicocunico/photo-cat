import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import "../../i18n";
import type { OpenedReview, ReviewVariantView, ShotDecision } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useCommandStore } from "../../commands/registry";
import { ModuleNav } from "../../components/shell/ModuleNav";
import { useLayout } from "../../store/layout";
import { AUTOSAVE_DELAY_MS, chooseWinner, setRating, useRawReview } from "../../store/rawReview";
import { useToast } from "../../store/toasts";
import { RawReviewPage } from "./RawReviewPage";
import { forgetMetadata } from "./useShotMetadata";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (key: string, scheme: string) => `${scheme}://${key}`,
}));

const variant = (preset: string, shot: string, over: Partial<ReviewVariantView> = {}) =>
  ({
    preset,
    fileName: `${shot}-${preset}.jpg`,
    extension: "jpg",
    bytes: 1,
    kind: "jpeg",
    selectable: true,
    exportable: true,
    ...over,
  }) as unknown as ReviewVariantView;

const PRESETS = ["Acros", "Velvia", "Provia"];

const review = (over: Partial<OpenedReview> = {}): OpenedReview =>
  ({
    sessionId: "s1",
    session: {
      schemaVersion: 2,
      name: "Dolomiti",
      sourceRoot: "D:/Foto/Dolomiti",
      layout: null,
      variantsOutputDir: null,
      presets: PRESETS,
      guidePreset: "Acros",
      panelPresets: PRESETS,
      hiddenPresets: [],
      decisions: {},
      lastShotKey: null,
      createdAt: "2025-01-10T09:00:00.000Z",
      updatedAt: "2025-01-10T09:00:00.000Z",
    },
    layout: "presets",
    presets: PRESETS,
    guidePreset: "Acros",
    shots: ["dscf0001", "dscf0002", "dscf0003"].map((key) => ({
      key,
      displayName: key.toUpperCase(),
      variants: PRESETS.map((p) => variant(p, key)),
    })),
    warnings: [],
    ...over,
  }) as OpenedReview;

/** Pagina, navigazione e comandi della shell, come li monta l'app. */
function Harness() {
  const module = useLayout((s) => s.module);
  return (
    <>
      <ModuleNav />
      {module === "raw-review" && <RawReviewPage />}
    </>
  );
}

function mount() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <Harness />
    </QueryClientProvider>,
  );
}

// Decisioni come le tiene il backend: ogni comando restituisce la decisione intera.
const backend = new Map<string, ShotDecision>();
const decisionOf = (key: string, over: Partial<ShotDecision> = {}): ShotDecision => {
  const next = {
    key,
    rating: 0,
    winnerPreset: null,
    reviewed: false,
    ...backend.get(key),
    ...over,
  };
  backend.set(key, next);
  return next;
};

const press = (key: string, init: KeyboardEventInit = {}) =>
  fireEvent.keyDown(window, { key, ...init });
const shiftDigit = (n: number) =>
  fireEvent.keyDown(window, { key: "!@#$%^&*("[n - 1], code: `Digit${n}`, shiftKey: true });
const altDigit = (n: number) =>
  fireEvent.keyDown(window, { key: "¡", code: `Digit${n}`, altKey: true });

beforeEach(() => {
  vi.clearAllMocks();
  backend.clear();
  useCommandStore.setState({ commands: [] });
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  })) as unknown as typeof window.matchMedia;
  useLayout.setState({ module: "raw-review", rawReviewTarget: null, navCompact: false });
  useRawReview.getState().clear();
  useToast.setState({ notices: [] });
  forgetMetadata("s1");
  commandsMock.rawReviewRecent.mockResolvedValue([]);
  commandsMock.rawReviewImageKeys.mockImplementation(async (_s: string, shot: string) =>
    PRESETS.map((preset) => ({ preset, key: `${shot}-${preset}` })),
  );
  commandsMock.rawReviewSetWinner.mockImplementation(
    async (_s: string, key: string, preset: string | null) =>
      decisionOf(key, { winnerPreset: preset }),
  );
  commandsMock.rawReviewSetReviewed.mockImplementation(
    async (_s: string, key: string, reviewed: boolean) => decisionOf(key, { reviewed }),
  );
  commandsMock.rawReviewSetRating.mockImplementation(
    async (_s: string, key: string, rating: number) => decisionOf(key, { rating }),
  );
});
afterEach(() => {
  vi.useRealTimers();
  cleanup();
});

async function openReview(opened = review()) {
  useRawReview.getState().setOpened(opened);
  const view = mount();
  await screen.findByRole("heading", { name: opened.session.name });
  return view;
}

const panel = (name: string) => screen.getByRole("region", { name });
const position = (text: string) => expect(screen.getByText(text)).toBeInTheDocument();

describe("scorciatoie della revisione", () => {
  it("Maiusc+2 e poi 4: il secondo pannello è il migliore con 4 stelle e lo scatto non cambia", async () => {
    await openReview();
    shiftDigit(2);
    press("4");
    await waitFor(() => expect(panel("Velvia")).toHaveClass("is-winner"));
    const decision = useRawReview.getState().opened!.session.decisions!.dscf0001;
    expect(decision).toMatchObject({ winnerPreset: "Velvia", rating: 4, reviewed: true });
    expect(commandsMock.rawReviewSetWinner).toHaveBeenCalledWith("s1", "dscf0001", "Velvia");
    expect(commandsMock.rawReviewSetRating).toHaveBeenCalledWith("s1", "dscf0001", 4);
    // Né la scelta né le stelle fanno avanzare.
    position("1 / 3 · DSCF0001");
    expect(useRawReview.getState().shotIndex).toBe(0);
    // Le stelle sono anche nella barra dello scatto.
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "4 stelle" })).toHaveAttribute(
        "aria-pressed",
        "true",
      ),
    );
  });

  it("Alt+1–5 e Alt+0 valutano come 1–5 e 0", async () => {
    await openReview();
    altDigit(3);
    await waitFor(() =>
      expect(commandsMock.rawReviewSetRating).toHaveBeenLastCalledWith("s1", "dscf0001", 3),
    );
    altDigit(0);
    await waitFor(() =>
      expect(commandsMock.rawReviewSetRating).toHaveBeenLastCalledWith("s1", "dscf0001", 0),
    );
    press("0");
    press("5");
    await waitFor(() =>
      expect(commandsMock.rawReviewSetRating).toHaveBeenLastCalledWith("s1", "dscf0001", 5),
    );
  });

  it("Spazio e Maiusc+Spazio passano allo scatto successivo e precedente", async () => {
    await openReview();
    press(" ");
    position("2 / 3 · DSCF0002");
    press(" ");
    position("3 / 3 · DSCF0003");
    press(" ");
    position("3 / 3 · DSCF0003");
    press(" ", { shiftKey: true });
    position("2 / 3 · DSCF0002");
    expect(commandsMock.rawReviewSetLastShot).toHaveBeenLastCalledWith("s1", "dscf0002");
  });

  it("Invio sceglie il pannello a fuoco e le frecce spostano il fuoco", async () => {
    await openReview();
    expect(panel("Acros")).toHaveAttribute("data-focused", "true");
    press("ArrowRight");
    press("ArrowRight");
    press("ArrowRight");
    expect(panel("Provia")).toHaveAttribute("data-focused", "true");
    press("Enter");
    await waitFor(() => expect(panel("Provia")).toHaveClass("is-winner"));
    expect(commandsMock.rawReviewSetWinner).toHaveBeenCalledWith("s1", "dscf0001", "Provia");
    press("ArrowLeft");
    expect(panel("Velvia")).toHaveAttribute("data-focused", "true");
    position("1 / 3 · DSCF0001");
  });

  it("Maiusc+N sull'Anteprima RAW avvisa senza chiamare il backend; oltre i pannelli non fa nulla", async () => {
    const opened = review();
    opened.layout = "rafWithVariants";
    opened.session.panelPresets = ["Acros", "raw-preview"];
    opened.shots = opened.shots.map((s) => ({
      ...s,
      variants: [
        variant("Acros", s.key),
        variant("raw-preview", s.key, { selectable: false, kind: "rawPreview" as never }),
      ],
    }));
    await openReview(opened);
    shiftDigit(2);
    expect(useToast.getState().notices).toHaveLength(1);
    expect(useToast.getState().notices[0].message).toMatch(/anteprima RAW/i);
    shiftDigit(5);
    expect(commandsMock.rawReviewSetWinner).not.toHaveBeenCalled();
    // Si valuta comunque.
    press("2");
    await waitFor(() => expect(commandsMock.rawReviewSetRating).toHaveBeenCalled());
  });

  it("le scorciatoie sono ignorate nei campi di testo e con un dialogo aperto", async () => {
    await openReview();
    const input = document.createElement("input");
    input.type = "text";
    document.body.appendChild(input);
    input.focus();
    for (const init of [
      { key: "3" },
      { key: " " },
      { key: "Enter" },
      { key: "ArrowRight" },
    ] as KeyboardEventInit[])
      fireEvent.keyDown(input, init);
    fireEvent.keyDown(input, { key: "@", code: "Digit2", shiftKey: true });
    input.remove();
    expect(commandsMock.rawReviewSetRating).not.toHaveBeenCalled();
    expect(commandsMock.rawReviewSetWinner).not.toHaveBeenCalled();
    position("1 / 3 · DSCF0001");

    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    document.body.appendChild(dialog);
    press("3");
    press(" ");
    dialog.remove();
    expect(commandsMock.rawReviewSetRating).not.toHaveBeenCalled();
    position("1 / 3 · DSCF0001");
  });

  it("le stelle della barra valutano lo scatto e il clic sulla stella attuale le toglie", async () => {
    await openReview();
    fireEvent.click(screen.getByRole("button", { name: "2 stelle" }));
    await waitFor(() =>
      expect(commandsMock.rawReviewSetRating).toHaveBeenCalledWith("s1", "dscf0001", 2),
    );
    expect(screen.getByRole("button", { name: "2 stelle" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: "2 stelle" }));
    await waitFor(() =>
      expect(commandsMock.rawReviewSetRating).toHaveBeenLastCalledWith("s1", "dscf0001", 0),
    );
  });

  it("una valutazione rifiutata dal backend torna al valore di prima e avvisa", async () => {
    await openReview();
    commandsMock.rawReviewSetRating.mockRejectedValue({ code: "INVALID_INPUT", detail: "no" });
    press("5");
    expect(useRawReview.getState().opened!.session.decisions!.dscf0001.rating).toBe(5);
    await waitFor(() => expect(useToast.getState().notices).toHaveLength(1));
    expect(useRawReview.getState().opened!.session.decisions!.dscf0001).toMatchObject({
      rating: 0,
      reviewed: false,
    });
  });

  it("Ctrl+3 apre Cataloghi e la sessione resta salvata, anche con una modifica in attesa", async () => {
    await openReview();
    press("4");
    expect(commandsMock.rawReviewSave).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: "3", ctrlKey: true });
    expect(useLayout.getState().module).toBe("catalogs");
    await waitFor(() => expect(commandsMock.rawReviewSave).toHaveBeenCalledWith("s1"));
    // La modifica è arrivata al backend prima del salvataggio.
    const order = (m: { mock: { invocationCallOrder: number[] } }) => m.mock.invocationCallOrder[0];
    expect(order(commandsMock.rawReviewSetRating)).toBeLessThan(
      order(commandsMock.rawReviewSave),
    );
    expect(commandsMock.rawReviewSave).toHaveBeenCalledTimes(1);
    expect(useRawReview.getState().opened?.sessionId).toBe("s1");
    // Il salvataggio automatico in attesa è stato annullato dal salvataggio immediato.
    await act(async () => {
      await new Promise((r) => setTimeout(r, AUTOSAVE_DELAY_MS + 100));
    });
    expect(commandsMock.rawReviewSave).toHaveBeenCalledTimes(1);
  });

  it("Ctrl+9 e i tasti dei moduli non sono toccati dal modulo", async () => {
    await openReview();
    fireEvent.keyDown(window, { key: "1", ctrlKey: true });
    expect(useLayout.getState().module).not.toBe("raw-review");
    expect(commandsMock.rawReviewSetRating).not.toHaveBeenCalled();
    expect(commandsMock.rawReviewSetWinner).not.toHaveBeenCalled();
  });

  it("fuori dal modulo le scorciatoie non sono registrate", async () => {
    const view = await openReview();
    expect(
      useCommandStore.getState().commands.some((c) => c.id.startsWith("rawReview.")),
    ).toBe(true);
    useLayout.setState({ module: "catalogs" });
    view.rerender(
      <QueryClientProvider client={new QueryClient()}>
        <Harness />
      </QueryClientProvider>,
    );
    await waitFor(() =>
      expect(
        useCommandStore.getState().commands.some((c) => c.id.startsWith("rawReview.")),
      ).toBe(false),
    );
  });

  it("il foglio delle scorciatoie descrive le scorciatoie del modulo", async () => {
    const { entries } = await import("../../commands/catalog");
    const i18n = (await import("../../i18n")).default;
    const docs = entries(i18n.t).filter((e) => e.group === "rawReview");
    expect(docs.map((d) => d.keys)).toEqual(
      expect.arrayContaining(["Maiusc+1…9", "Invio", "Spazio, Maiusc+Spazio"]),
    );
    expect(docs.every((d) => d.label && !d.label.startsWith("shortcuts."))).toBe(true);
  });
});

describe("salvataggio automatico", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    useRawReview.getState().setOpened(review());
  });

  it("salva una volta sola dopo una pausa di mezzo secondo dall'ultima modifica", async () => {
    await act(async () => {
      void setRating("dscf0001", 3);
    });
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS - 100);
    expect(commandsMock.rawReviewSave).not.toHaveBeenCalled();
    // Un'altra modifica fa ripartire l'attesa.
    await act(async () => {
      void chooseWinner("dscf0001", "Velvia");
    });
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS - 100);
    expect(commandsMock.rawReviewSave).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(150);
    expect(commandsMock.rawReviewSave).toHaveBeenCalledTimes(1);
    expect(commandsMock.rawReviewSave).toHaveBeenCalledWith("s1");
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS * 3);
    expect(commandsMock.rawReviewSave).toHaveBeenCalledTimes(1);
  });

  it("anche cambiare scatto riavvia il salvataggio automatico", async () => {
    const { selectShot } = await import("../../store/rawReview");
    await act(async () => selectShot(1));
    await vi.advanceTimersByTimeAsync(AUTOSAVE_DELAY_MS + 10);
    expect(commandsMock.rawReviewSave).toHaveBeenCalledTimes(1);
  });

  it("la scelta non avanza e due modifiche in fila arrivano al backend nell'ordine", async () => {
    const calls: string[] = [];
    commandsMock.rawReviewSetWinner.mockImplementation(async (_s, key, preset) => {
      await new Promise((r) => setTimeout(r, 50));
      calls.push(`winner:${preset}`);
      return decisionOf(key, { winnerPreset: preset });
    });
    commandsMock.rawReviewSetRating.mockImplementation(async (_s, key, rating) => {
      calls.push(`rating:${rating}`);
      return decisionOf(key, { rating });
    });
    const first = chooseWinner("dscf0001", "Velvia");
    const second = setRating("dscf0001", 4);
    await vi.advanceTimersByTimeAsync(100);
    await Promise.all([first, second]);
    expect(calls).toEqual(["winner:Velvia", "rating:4"]);
    expect(useRawReview.getState().shotIndex).toBe(0);
    expect(useRawReview.getState().opened!.session.decisions!.dscf0001).toMatchObject({
      winnerPreset: "Velvia",
      rating: 4,
      reviewed: true,
    });
  });
});

