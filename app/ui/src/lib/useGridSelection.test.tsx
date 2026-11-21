import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useGridSelection } from "./useGridSelection";

type Scope = { group: string };
const order = [10, 11, 12, 13, 14];
const setup = (fetchBase = vi.fn().mockResolvedValue({ count: 400, maxId: 999 }), onError?: () => void) =>
  renderHook(() =>
    useGridSelection<Scope>({ filter: { group: "g" }, order, fetchBase, onError }),
  );

describe("useGridSelection", () => {
  it("clic, Ctrl+clic e Maiusc+clic con ancora", () => {
    const { result } = setup();
    act(() => result.current.click(11, { ctrl: false, shift: false }));
    expect(result.current.ids).toEqual([11]);
    act(() => result.current.click(13, { ctrl: false, shift: true }));
    expect(result.current.ids).toEqual([11, 12, 13]);
    act(() => result.current.click(14, { ctrl: true, shift: false }));
    expect(result.current.count).toBe(4);
    expect(result.current.has(14)).toBe(true);
  });

  it("Seleziona tutto usa la base del backend senza caricare gli id", async () => {
    const { result } = setup();
    await act(() => result.current.selectAll());
    expect(result.current.selection).toMatchObject({
      mode: "filter",
      filter: { group: "g" },
      maxId: 999,
      total: 400,
    });
    expect(result.current.count).toBe(400);
    expect(result.current.has(5)).toBe(true);
    expect(result.current.has(1000)).toBe(false);
  });

  it("Ctrl+clic su una selezione per filtro aggiunge eccezioni", async () => {
    const { result } = setup();
    await act(() => result.current.selectAll());
    act(() => result.current.click(12, { ctrl: true, shift: false }));
    expect(result.current.count).toBe(399);
    expect(result.current.has(12)).toBe(false);
    act(() => result.current.click(12, { ctrl: true, shift: false }));
    expect(result.current.count).toBe(400);
  });

  it("Inverti scambia scelti ed eccezioni, in entrambi i versi", async () => {
    const { result } = setup();
    act(() => result.current.setIds([10, 11], 10));
    await act(() => result.current.invert());
    expect(result.current.count).toBe(398);
    expect(result.current.has(10)).toBe(false);
    expect(result.current.has(12)).toBe(true);
    await act(() => result.current.invert());
    expect(result.current.ids).toEqual([10, 11]);
  });

  it("l'errore del backend lascia la selezione e arriva a onError", async () => {
    const onError = vi.fn();
    const { result } = setup(vi.fn().mockRejectedValue(new Error("boom")), onError);
    act(() => result.current.setIds([10]));
    await act(() => result.current.selectAll());
    await act(() => result.current.invert());
    expect(onError).toHaveBeenCalledTimes(2);
    expect(result.current.ids).toEqual([10]);
  });

  it("clear svuota e azzera l'ancora (Esc)", async () => {
    const { result } = setup();
    await act(() => result.current.selectAll());
    act(() => result.current.clear());
    expect(result.current.count).toBe(0);
    expect(result.current.ids).toEqual([]);
    // senza ancora, Maiusc+clic è un clic semplice
    act(() => result.current.click(13, { ctrl: false, shift: true }));
    expect(result.current.ids).toEqual([13]);
  });
});
