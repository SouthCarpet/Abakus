import '@testing-library/jest-dom/vitest'

// jsdom has neither ResizeObserver nor matchMedia. Recharts' ResponsiveContainer
// needs the former to ever report a non-zero size (it bails out to 0x0 without
// it, so nothing draws), and its animations check the latter for
// prefers-reduced-motion; stubbing "reduced" resolves them synchronously so
// chart render tests can find real DOM nodes without waiting on animation frames.
class TestResizeObserver {
  private readonly callback: ResizeObserverCallback
  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
  }
  observe(target: Element) {
    this.callback([{ target, contentRect: { width: 400, height: 220 } } as ResizeObserverEntry], this as unknown as ResizeObserver)
  }
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver = TestResizeObserver as unknown as typeof ResizeObserver
Element.prototype.getBoundingClientRect = () =>
  ({ width: 400, height: 220, top: 0, left: 0, bottom: 0, right: 0, x: 0, y: 0, toJSON: () => ({}) }) as DOMRect
globalThis.matchMedia = ((query: string) => ({
  matches: true,
  media: query,
  onchange: null,
  addEventListener: () => {},
  removeEventListener: () => {},
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false,
})) as unknown as typeof matchMedia
