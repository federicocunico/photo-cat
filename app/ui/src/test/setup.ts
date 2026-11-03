import "@testing-library/jest-dom/vitest";

// jsdom non implementa queste API, usate dalle primitive Radix (menu, popover, dialoghi).
if (typeof window !== "undefined") {
  class RO {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  window.ResizeObserver ??= RO as unknown as typeof ResizeObserver;
  const proto = window.HTMLElement.prototype as unknown as Record<string, unknown>;
  proto.hasPointerCapture ??= () => false;
  proto.setPointerCapture ??= () => {};
  proto.releasePointerCapture ??= () => {};
  proto.scrollIntoView ??= () => {};
}

// Gli avvisi vivono in uno store globale: ogni test parte senza avvisi.
import { afterEach } from "vitest";
import { useToast } from "../store/toasts";
afterEach(() => useToast.setState({ notices: [] }));

// Guida: ogni test parte con i suggerimenti già visti (i test della guida li riattivano)
// e senza passi completati, così i suggerimenti non coprono i moduli negli altri test.
// Import dinamico: lo store non deve caricare `lib/uiPrefs` prima dei mock del test.
import { beforeEach } from "vitest";
beforeEach(async () => {
  const { useGuide } = await import("../store/guide");
  useGuide.setState({
    done: {},
    hidden: false,
    seen: ["people", "catalogs", "tags", "sequence", "moved"],
    session: [],
    active: null,
    intro: false,
    guide: null,
    report: false,
  });
});
