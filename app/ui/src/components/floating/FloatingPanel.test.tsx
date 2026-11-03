import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import "../../i18n";
import { openers, useFloating } from "../../store/floating";
import { FloatingPanel } from "./FloatingPanel";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

const rect = (id: string) => {
  const p = useFloating.getState().panels[id];
  return { x: p.x, y: p.y, w: p.w, h: p.h };
};

function pointer(type: string, el: Element, x: number, y: number) {
  // jsdom non ha sempre PointerEvent: un MouseEvent con pointerId basta ai gestori.
  const e = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: x,
    clientY: y,
    button: 0,
  });
  Object.defineProperty(e, "pointerId", { value: 1 });
  fireEvent(el, e);
}

function Panels({ ids }: { ids: string[] }) {
  return (
    <>
      {ids.map((id) => (
        <FloatingPanel key={id} id={id} title={`Pannello ${id}`}>
          <p>contenuto {id}</p>
        </FloatingPanel>
      ))}
    </>
  );
}

beforeEach(() => {
  localStorage.clear();
  useFloating.setState({ panels: {}, order: [] });
  Object.assign(window, { innerWidth: 1200, innerHeight: 800 });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

describe("pannello flottante", () => {
  it("si trascina dalla barra del titolo e si ridimensiona dall'angolo", () => {
    act(() => useFloating.getState().open("a", { x: 100, y: 100, w: 320, h: 300 }));
    render(<Panels ids={["a"]} />);
    expect(screen.getByRole("dialog", { name: "Pannello a" })).toHaveAttribute(
      "aria-modal",
      "false",
    );
    const title = screen.getByTestId("floating-a-title");
    pointer("pointerdown", title, 110, 110);
    pointer("pointermove", title, 160, 140);
    pointer("pointerup", title, 160, 140);
    expect(rect("a")).toEqual({ x: 150, y: 130, w: 320, h: 300 });
    const corner = screen.getByTestId("floating-a-resize");
    pointer("pointerdown", corner, 470, 430);
    pointer("pointermove", corner, 520, 480);
    pointer("pointerup", corner, 520, 480);
    expect(rect("a")).toEqual({ x: 150, y: 130, w: 370, h: 350 });
    // Mai sotto la dimensione minima né fuori dalla finestra.
    pointer("pointerdown", title, 160, 140);
    pointer("pointermove", title, 5000, 5000);
    pointer("pointerup", title, 5000, 5000);
    expect(rect("a")).toEqual({ x: 1200 - 370, y: 800 - 350, w: 370, h: 350 });
  });

  it("resta dentro la finestra quando questa si rimpicciolisce", () => {
    act(() => useFloating.getState().open("a", { x: 800, y: 500, w: 360, h: 280 }));
    render(<Panels ids={["a"]} />);
    act(() => {
      Object.assign(window, { innerWidth: 900, innerHeight: 600 });
      window.dispatchEvent(new Event("resize"));
    });
    expect(rect("a")).toEqual({ x: 540, y: 320, w: 360, h: 280 });
    act(() => {
      Object.assign(window, { innerWidth: 300, innerHeight: 260 });
      window.dispatchEvent(new Event("resize"));
    });
    expect(rect("a")).toEqual({ x: 0, y: 0, w: 300, h: 260 });
  });

  it("il pannello cliccato va in primo piano", () => {
    act(() => {
      useFloating.getState().open("a", { x: 0, y: 0, w: 300, h: 300 });
      useFloating.getState().open("b", { x: 40, y: 40, w: 300, h: 300 });
    });
    render(<Panels ids={["a", "b"]} />);
    // Livello sopra `--pc-z-floating` (il valore assoluto viene dai token, non da numeri fissi).
    const z = (id: string) =>
      Number(screen.getByTestId(`floating-${id}`).style.getPropertyValue("--floating-layer"));
    expect(z("b")).toBeGreaterThan(z("a"));
    pointer("pointerdown", screen.getByText("contenuto a"), 10, 10);
    expect(z("a")).toBeGreaterThan(z("b"));
  });

  it("Esc chiude il pannello e riporta il focus al comando che lo ha aperto", () => {
    render(
      <>
        <button type="button">Apri</button>
        <Panels ids={["a"]} />
      </>,
    );
    const opener = screen.getByRole("button", { name: "Apri" });
    act(() => {
      openers.set("a", opener);
      useFloating.getState().open("a", { x: 0, y: 0, w: 300, h: 300 });
    });
    const dialog = screen.getByRole("dialog", { name: "Pannello a" });
    expect(document.activeElement).toBe(dialog);
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(document.activeElement).toBe(opener);
  });

  it("posizione e dimensione restano dopo un nuovo montaggio", () => {
    act(() => useFloating.getState().open("a", { x: 10, y: 10, w: 300, h: 300 }));
    const { unmount } = render(<Panels ids={["a"]} />);
    act(() => useFloating.getState().setRect("a", { x: 222, y: 111, w: 333, h: 444 }));
    act(() => useFloating.getState().close("a"));
    unmount();
    const saved = JSON.parse(localStorage.getItem("photo-cat.floating") ?? "{}");
    expect(saved.state.panels.a).toMatchObject({ x: 222, y: 111, w: 333, h: 444, open: false });
    // Riapertura (anche dopo un riavvio): la posizione salvata vince su quella iniziale.
    act(() => useFloating.getState().open("a", { x: 0, y: 0, w: 300, h: 300 }));
    render(<Panels ids={["a"]} />);
    expect(rect("a")).toEqual({ x: 222, y: 111, w: 333, h: 444 });
    expect(screen.getByTestId("floating-a").style.left).toBe("222px");
  });
  it("ha un'etichetta di chiusura propria e usa il livello dei token", () => {
    act(() => useFloating.getState().open("a", { x: 0, y: 0, w: 300, h: 300 }));
    render(<Panels ids={["a"]} />);
    expect(screen.getByRole("button", { name: "Chiudi Pannello a" })).toBeInTheDocument();
    expect(screen.getByTestId("floating-a")).toHaveClass("floating-panel");
    // Il livello è in floating.css: calc(var(--pc-z-floating) + var(--floating-layer)).
    expect(screen.getByTestId("floating-a").style.zIndex).toBe("");
    expect(screen.getByTestId("floating-a").style.getPropertyValue("--floating-layer")).toBe("0");
  });

  it("Pannello flottante senza mouse: Sposta e Ridimensiona dal menu, poi frecce", async () => {
    const user = userEvent.setup();
    act(() => useFloating.getState().open("a", { x: 100, y: 100, w: 320, h: 300 }));
    render(<Panels ids={["a"]} />);
    await user.click(screen.getByRole("button", { name: "Opzioni di Pannello a" }));
    expect(screen.getByRole("menuitem", { name: /Sposta/ })).toHaveTextContent("Frecce");
    expect(screen.getByRole("menuitem", { name: /Chiudi/ })).toHaveTextContent("Esc");
    await user.click(screen.getByRole("menuitem", { name: /Sposta/ }));
    expect(screen.getByRole("status")).toHaveTextContent("Sposta con le frecce");
    await waitFor(() =>
      expect(screen.getByTestId("floating-a")).toContainElement(
        document.activeElement as HTMLElement,
      ),
    );
    await user.keyboard("{ArrowRight}{ArrowDown}");
    expect(rect("a")).toEqual({ x: 110, y: 110, w: 320, h: 300 });
    await user.keyboard("{Shift>}{ArrowLeft}{/Shift}");
    expect(rect("a")).toEqual({ x: 60, y: 110, w: 320, h: 300 });
    // Esc esce dalla modalità senza chiudere il pannello.
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Pannello a" })).toBeInTheDocument();
    await user.keyboard("{ArrowRight}");
    expect(rect("a")).toEqual({ x: 60, y: 110, w: 320, h: 300 });

    await user.click(screen.getByRole("button", { name: "Opzioni di Pannello a" }));
    await user.click(screen.getByRole("menuitem", { name: /Ridimensiona/ }));
    await waitFor(() =>
      expect(screen.getByTestId("floating-a")).toContainElement(
        document.activeElement as HTMLElement,
      ),
    );
    await user.keyboard("{ArrowRight}{Shift>}{ArrowDown}{/Shift}");
    expect(rect("a")).toEqual({ x: 60, y: 110, w: 330, h: 350 });
    // Mai sotto la dimensione minima.
    for (let i = 0; i < 4; i++) await user.keyboard("{Shift>}{ArrowLeft}{/Shift}");
    expect(rect("a").w).toBe(260);
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
