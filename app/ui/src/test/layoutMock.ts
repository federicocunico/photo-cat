// jsdom non calcola il layout: forniamo dimensioni fisse e un ResizeObserver
// minimale perché i componenti virtualizzati rendano qualcosa nei test.

const SIZE = 800;

Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
  configurable: true,
  get: () => SIZE,
});
Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
  configurable: true,
  get: () => SIZE,
});
HTMLElement.prototype.getBoundingClientRect = function () {
  return {
    x: 0,
    y: 0,
    top: 0,
    left: 0,
    right: SIZE,
    bottom: SIZE,
    width: SIZE,
    height: SIZE,
    toJSON: () => ({}),
  };
};

class FakeResizeObserver {
  constructor(private cb: ResizeObserverCallback) {}
  observe(target: Element) {
    const rect = target.getBoundingClientRect();
    this.cb(
      [{ target, contentRect: rect } as unknown as ResizeObserverEntry],
      this as unknown as ResizeObserver,
    );
  }
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= FakeResizeObserver as unknown as typeof ResizeObserver;
Object.defineProperty(Element.prototype, "clientHeight", { configurable: true, get: () => SIZE });
Object.defineProperty(Element.prototype, "clientWidth", { configurable: true, get: () => SIZE });
