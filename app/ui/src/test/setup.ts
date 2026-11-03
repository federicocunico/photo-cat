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
