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
