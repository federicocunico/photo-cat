import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import "../../i18n";
import type { ImageMetadata, ReviewShotView, ReviewVariantView } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useToast } from "../../store/toasts";
import { ComparePanels } from "./ComparePanels";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (key: string, scheme: string) => `${scheme}://${key}`,
}));

// jsdom non ha PointerEvent: serve con le coordinate del mouse per il trascinamento.
if (typeof window.PointerEvent === "undefined") {
  class TestPointerEvent extends MouseEvent {
    pointerId: number;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
    }
  }
  Object.defineProperty(window, "PointerEvent", { value: TestPointerEvent, configurable: true });
}

const variant = (preset: string, over: Partial<ReviewVariantView> = {}): ReviewVariantView => ({
  preset,
  fileName: `DSCF0001-${preset}.jpg`,
  extension: "jpg",
  bytes: 1_000_000,
  kind: "jpeg" as ReviewVariantView["kind"],
  selectable: true,
  exportable: true,
  ...over,
});

const shotWith = (presets: string[], key = "dscf0001"): ReviewShotView => ({
  key,
  displayName: key.toUpperCase(),
  variants: presets.map((p) => variant(p)),
});

const metadata = (summary: string): ImageMetadata => ({
  sections: [],
  gps: null,
  recipe: {
    summary,
    film: "Velvia",
    filmKey: null,
    dynamicRange: "DR200",
    whiteBalance: "Auto",
    whiteBalanceKey: "wbAuto",
    rows: [{ label: "filmSimulation", value: "Velvia", valueKey: null }],
  },
});

function mount(
  props: Partial<Parameters<typeof ComparePanels>[0]> & { panels: string[] },
  shot: ReviewShotView = shotWith(props.panels),
) {
  const onChoose = vi.fn();
  const onHide = vi.fn();
  const utils = render(
    <ComparePanels
      sessionId="s1"
      shot={shot}
      winnerPreset={null}
      metadata={{}}
      onChoose={onChoose}
      onHide={onHide}
      {...props}
    />,
  );
  return { ...utils, onChoose, onHide, shot };
}

/** Il contenitore trasformato di ogni pannello: l'immagine con il suo `transform`. */
const transforms = () =>
  screen.getAllByRole("img").map((img) => (img as HTMLElement).style.transform);

beforeEach(() => {
  vi.clearAllMocks();
  useToast.setState({ notices: [] });
  commandsMock.rawReviewImageKeys.mockImplementation(async (_s: string, shot: string) =>
    ["a", "b", "c", "d", "raw-preview"].map((preset) => ({ preset, key: `${shot}-${preset}` })),
  );
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("griglia dei pannelli", () => {
  it.each([
    [1, "repeat(1, minmax(0, 1fr))", "repeat(1, minmax(0, 1fr))"],
    [2, "repeat(2, minmax(0, 1fr))", "repeat(1, minmax(0, 1fr))"],
    [4, "repeat(2, minmax(0, 1fr))", "repeat(2, minmax(0, 1fr))"],
    [5, "repeat(3, minmax(0, 1fr))", "repeat(2, minmax(0, 1fr))"],
    [9, "repeat(3, minmax(0, 1fr))", "repeat(3, minmax(0, 1fr))"],
  ])("%i pannelli", async (count, columns, rows) => {
    const panels = Array.from({ length: count }, (_, i) => `p${i}`);
    mount({ panels });
    const region = screen.getByRole("group", { name: "Confronto delle varianti" });
    expect(region.style.gridTemplateColumns).toBe(columns);
    expect(region.style.gridTemplateRows).toBe(rows);
    expect(screen.getAllByRole("region")).toHaveLength(count);
  });
});

describe("anteprime", () => {
  it("chiede le chiavi al lato del precaricamento e usa il protocollo review", async () => {
    mount({ panels: ["a", "b"] });
    const images = await screen.findAllByRole("img");
    expect(commandsMock.rawReviewImageKeys).toHaveBeenCalledWith("s1", "dscf0001", 2048);
    expect(images.map((i) => i.getAttribute("src"))).toEqual([
      "review://dscf0001-a",
      "review://dscf0001-b",
    ]);
  });

  it("chiede un lato maggiore quando lo zoom lo richiede", async () => {
    mount({ panels: ["a"] });
    await screen.findAllByRole("img");
    const viewport = document.querySelector(".rr-viewport") as HTMLElement;
    for (let i = 0; i < 9; i += 1) fireEvent.wheel(viewport, { deltaY: -100 });
    await act(async () => {});
    const edges = commandsMock.rawReviewImageKeys.mock.calls.map((c) => c[2]);
    expect(edges[0]).toBe(2048);
    expect(edges[edges.length - 1]).toBeGreaterThan(2048);
  });
});

describe("zoom e spostamento sincronizzati", () => {
  it("la rotellina su un pannello ingrandisce tutti allo stesso modo", async () => {
    mount({ panels: ["a", "b", "c"] });
    await screen.findAllByRole("img");
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(1)"]));
    const second = document.querySelectorAll(".rr-viewport")[1] as HTMLElement;
    fireEvent.wheel(second, { deltaY: -100 });
    const after = transforms();
    expect(after).toHaveLength(3);
    expect(new Set(after).size).toBe(1);
    expect(after[0]).toBe("translate3d(0px, 0px, 0) scale(1.15)");
  });

  it("la rotellina in senso opposto torna alla vista adattata e non va sotto 1", async () => {
    mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    const first = document.querySelector(".rr-viewport") as HTMLElement;
    fireEvent.wheel(first, { deltaY: -100 });
    fireEvent.wheel(first, { deltaY: 100 });
    fireEvent.wheel(first, { deltaY: 100 });
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(1)"]));
  });

  it("trascinare un pannello ingrandito sposta tutti i pannelli insieme", async () => {
    mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    const [first, second] = Array.from(document.querySelectorAll(".rr-viewport")) as HTMLElement[];
    fireEvent.doubleClick(first);
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(2)"]));
    fireEvent.pointerDown(second, { clientX: 100, clientY: 100, pointerId: 1 });
    fireEvent.pointerMove(second, { clientX: 130, clientY: 90, pointerId: 1 });
    fireEvent.pointerUp(second, { pointerId: 1 });
    expect(new Set(transforms())).toEqual(new Set(["translate3d(30px, -10px, 0) scale(2)"]));
  });

  it("senza zoom il trascinamento non sposta nulla", async () => {
    mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    const first = document.querySelector(".rr-viewport") as HTMLElement;
    fireEvent.pointerDown(first, { clientX: 10, clientY: 10, pointerId: 1 });
    fireEvent.pointerMove(first, { clientX: 60, clientY: 60, pointerId: 1 });
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(1)"]));
  });

  it("il doppio clic adatta se è ingrandito e non sceglie la variante", async () => {
    const { onChoose } = mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    const first = document.querySelector(".rr-viewport") as HTMLElement;
    fireEvent.click(first, { detail: 1 });
    fireEvent.click(first, { detail: 2 });
    fireEvent.doubleClick(first);
    expect(onChoose).toHaveBeenCalledTimes(1);
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(2)"]));
    fireEvent.doubleClick(first);
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(1)"]));
  });

  it("un altro scatto riparte dalla vista adattata", async () => {
    const view = mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    fireEvent.doubleClick(document.querySelector(".rr-viewport") as HTMLElement);
    expect(transforms()[0]).toContain("scale(2)");
    view.rerender(
      <ComparePanels
        sessionId="s1"
        shot={shotWith(["a", "b"], "dscf0002")}
        panels={["a", "b"]}
        winnerPreset={null}
        metadata={{}}
        onChoose={view.onChoose}
        onHide={view.onHide}
      />,
    );
    await screen.findAllByRole("img");
    expect(new Set(transforms())).toEqual(new Set(["translate3d(0px, 0px, 0) scale(1)"]));
  });
});

describe("variante mancante e anteprima RAW", () => {
  it("indica «Variante non disponibile» nel pannello senza file", async () => {
    const shot = shotWith(["a"]);
    mount({ panels: ["a", "b"] }, shot);
    await screen.findAllByRole("img");
    const missing = screen.getByRole("region", { name: "b" });
    expect(within(missing).getByText("Variante non disponibile")).toBeInTheDocument();
    expect(within(missing).queryByRole("img")).toBeNull();
    // Niente da scegliere: né il pulsante né il clic.
    expect(within(missing).queryByRole("button", { name: /Scegli/ })).toBeNull();
  });

  it("l'anteprima RAW non si sceglie e spiega perché", async () => {
    const shot: ReviewShotView = {
      key: "dscf0001",
      displayName: "DSCF0001",
      variants: [variant("raw-preview", { selectable: false, exportable: false })],
    };
    const { onChoose } = mount({ panels: ["raw-preview"] }, shot);
    await screen.findAllByRole("img");
    const panel = screen.getByRole("region", { name: "Anteprima RAW" });
    fireEvent.click(within(panel).getByRole("button", { name: /Scegli «Anteprima RAW»/ }));
    fireEvent.click(panel.querySelector(".rr-viewport") as HTMLElement, { detail: 1 });
    expect(onChoose).not.toHaveBeenCalled();
    const notices = useToast.getState().notices;
    expect(notices).toHaveLength(1);
    expect(notices[0].message).toContain("sviluppa lo scatto");
  });
});

describe("scelta, evidenza e intestazione", () => {
  it("evidenzia la variante scelta con il badge e lo stato premuto", async () => {
    mount({ panels: ["a", "b"], winnerPreset: "b" });
    await screen.findAllByRole("img");
    const chosen = screen.getByRole("region", { name: "b" });
    expect(chosen).toHaveClass("is-winner");
    expect(within(chosen).getByText("Variante migliore")).toBeInTheDocument();
    expect(within(chosen).getByRole("button", { name: /Scegli «b»/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    const other = screen.getByRole("region", { name: "a" });
    expect(other).not.toHaveClass("is-winner");
    expect(within(other).queryByText("Variante migliore")).toBeNull();
  });

  it("sceglie con il pulsante e con il clic sul pannello", async () => {
    const { onChoose } = mount({ panels: ["a", "b"] });
    await screen.findAllByRole("img");
    const panel = screen.getByRole("region", { name: "a" });
    fireEvent.click(within(panel).getByRole("button", { name: /Scegli «a»/ }));
    fireEvent.click(panel.querySelector(".rr-viewport") as HTMLElement, { detail: 1 });
    expect(onChoose).toHaveBeenCalledTimes(2);
    expect(onChoose).toHaveBeenCalledWith("a");
  });

  it("mostra nome e riepilogo della ricetta nell'intestazione", async () => {
    mount({ panels: ["a"], metadata: { a: metadata("Velvia · DR200 · Auto") } });
    const panel = screen.getByRole("region", { name: "a" });
    expect(within(panel).getByText("Velvia · DR200 · Auto")).toBeInTheDocument();
  });

  it("nasconde un pannello, ma non l'ultimo", async () => {
    const { onHide, rerender, shot } = mount({ panels: ["a", "b"] });
    fireEvent.click(screen.getByRole("button", { name: "Nascondi il pannello «b»" }));
    expect(onHide).toHaveBeenCalledWith("b");
    rerender(
      <ComparePanels
        sessionId="s1"
        shot={shot}
        panels={["a"]}
        winnerPreset={null}
        metadata={{}}
        onChoose={vi.fn()}
        onHide={onHide}
      />,
    );
    const last = screen.getByRole("button", { name: "Nascondi il pannello «a»" });
    expect(last).toHaveAttribute("aria-disabled", "true");
    expect(last).toHaveAccessibleDescription(/Deve restare almeno un pannello visibile/);
  });
});

describe("precaricamento", () => {
  it("precarica gli scatti vicini con i preset visibili dopo 60 ms", async () => {
    vi.useFakeTimers();
    mount({ panels: ["c", "a"] });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(59);
    });
    expect(commandsMock.rawReviewPrefetch).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(commandsMock.rawReviewPrefetch).toHaveBeenCalledTimes(1);
    expect(commandsMock.rawReviewPrefetch).toHaveBeenCalledWith("s1", "dscf0001", ["c", "a"]);
  });

  it("scorrendo in fretta accoda solo l'ultimo scatto", async () => {
    vi.useFakeTimers();
    const view = mount({ panels: ["a", "b"] });
    const rerenderWith = (key: string) =>
      view.rerender(
        <ComparePanels
          sessionId="s1"
          shot={shotWith(["a", "b"], key)}
          panels={["a", "b"]}
          winnerPreset={null}
          metadata={{}}
          onChoose={view.onChoose}
          onHide={view.onHide}
        />,
      );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30);
    });
    rerenderWith("dscf0002");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30);
    });
    rerenderWith("dscf0003");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    expect(commandsMock.rawReviewPrefetch).toHaveBeenCalledTimes(1);
    expect(commandsMock.rawReviewPrefetch).toHaveBeenCalledWith("s1", "dscf0003", ["a", "b"]);
  });

  it("cambiando i pannelli visibili precarica i nuovi preset", async () => {
    vi.useFakeTimers();
    const view = mount({ panels: ["a", "b"] });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    view.rerender(
      <ComparePanels
        sessionId="s1"
        shot={view.shot}
        panels={["b"]}
        winnerPreset={null}
        metadata={{}}
        onChoose={view.onChoose}
        onHide={view.onHide}
      />,
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    expect(commandsMock.rawReviewPrefetch).toHaveBeenLastCalledWith("s1", "dscf0001", ["b"]);
  });
});
