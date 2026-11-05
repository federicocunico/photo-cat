import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorldMap } from "./WorldMap";
import { pixelToLonLat } from "./projection";

const SIZE = { width: 800, height: 600 };

function setup(props: Partial<React.ComponentProps<typeof WorldMap>> = {}) {
  const onViewChange = vi.fn();
  const onMapClick = vi.fn();
  render(
    <WorldMap
      zoomInLabel="Zoom in"
      zoomOutLabel="Zoom out"
      label="Map"
      initialView={{ centerLon: 0, centerLat: 0, zoom: 2 }}
      onViewChange={onViewChange}
      onMapClick={onMapClick}
      {...props}
    />,
  );
  return { map: screen.getByTestId("world-map"), onViewChange, onMapClick };
}
const zoomOf = () => Number(screen.getByTestId("world-map").getAttribute("data-zoom"));

if (typeof window.PointerEvent === "undefined") {
  class PointerEventPolyfill extends MouseEvent {
    pointerId: number;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
    }
  }
  vi.stubGlobal("PointerEvent", PointerEventPolyfill);
}

describe("WorldMap", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("zooms with the buttons and respects limits", () => {
    const { onViewChange } = setup();
    fireEvent.click(screen.getByLabelText("Zoom in"));
    expect(zoomOf()).toBe(4);
    expect(onViewChange).toHaveBeenCalledTimes(1);
    expect(onViewChange.mock.calls[0][1]).toHaveProperty("north");
    for (let i = 0; i < 10; i++) fireEvent.click(screen.getByLabelText("Zoom in"));
    expect(zoomOf()).toBe(64);
    expect(screen.getByLabelText("Zoom in")).toBeDisabled();
  });

  it("handles keyboard pan and zoom, notifying after idle", () => {
    const { map, onViewChange } = setup();
    fireEvent.keyDown(map, { key: "+" });
    expect(zoomOf()).toBe(4);
    fireEvent.keyDown(map, { key: "ArrowRight" });
    expect(onViewChange).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(160);
    });
    expect(onViewChange).toHaveBeenCalledTimes(1);
    expect(onViewChange.mock.calls[0][0].centerLon).toBeGreaterThan(0);
  });

  it("wheel zooms and fires once after idle", () => {
    const { map, onViewChange } = setup();
    fireEvent.wheel(map, { deltaY: -200, clientX: 400, clientY: 300 });
    fireEvent.wheel(map, { deltaY: -200, clientX: 400, clientY: 300 });
    expect(zoomOf()).toBeGreaterThan(2);
    expect(onViewChange).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(160);
    });
    expect(onViewChange).toHaveBeenCalledTimes(1);
  });

  it("drag pans and fires on pointer up without a click", () => {
    const { map, onViewChange, onMapClick } = setup();
    fireEvent.pointerDown(map, { pointerId: 1, button: 0, clientX: 400, clientY: 300 });
    fireEvent.pointerMove(map, { pointerId: 1, clientX: 500, clientY: 300 });
    fireEvent.pointerUp(map, { pointerId: 1, clientX: 500, clientY: 300 });
    expect(onViewChange).toHaveBeenCalledTimes(1);
    expect(onViewChange.mock.calls[0][0].centerLon).toBeLessThan(0);
    expect(onMapClick).not.toHaveBeenCalled();
  });

  it("click reports lon/lat through unproject", () => {
    const { map, onMapClick } = setup();
    fireEvent.pointerDown(map, { pointerId: 1, button: 0, clientX: 600, clientY: 200 });
    fireEvent.pointerUp(map, { pointerId: 1, clientX: 600, clientY: 200 });
    const [lon, lat] = pixelToLonLat({ centerLon: 0, centerLat: 0, zoom: 2 }, SIZE, 600, 200);
    expect(onMapClick).toHaveBeenCalledTimes(1);
    expect(onMapClick.mock.calls[0][0]).toBeCloseTo(lon, 6);
    expect(onMapClick.mock.calls[0][1]).toBeCloseTo(lat, 6);
  });
});

describe("WorldMap accessibility", () => {
  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("Enter acts as a click at the centre of the map, not on the zoom buttons", () => {
    const { map, onMapClick } = setup({ description: "Use the arrow keys" });
    expect(map).toHaveAccessibleDescription("Use the arrow keys");
    fireEvent.keyDown(screen.getByLabelText("Zoom in"), { key: "Enter" });
    expect(onMapClick).not.toHaveBeenCalled();
    fireEvent.keyDown(map, { key: "Enter" });
    expect(onMapClick).toHaveBeenCalledTimes(1);
    const [lon, lat, px] = onMapClick.mock.calls[0];
    expect(lon).toBeCloseTo(0, 6);
    expect(lat).toBeCloseTo(0, 6);
    expect(px).toEqual({ x: SIZE.width / 2, y: SIZE.height / 2 });
  });

  it("the wheel does not scroll the page (non-passive listener)", () => {
    const { map } = setup();
    const notCancelled = fireEvent.wheel(map, { deltaY: -100, clientX: 400, clientY: 300 });
    expect(notCancelled).toBe(false);
  });
});
