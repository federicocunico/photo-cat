import { cleanup, createEvent, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BarChart, ColumnSeries, Histogram, StackedBar } from ".";
import type { HistogramBin } from ".";

afterEach(cleanup);

/** jsdom non ha PointerEvent: clientX va assegnato a mano. */
const pointer = (
  el: Element,
  type: "pointerDown" | "pointerMove" | "pointerUp",
  clientX: number,
) => {
  const ev = createEvent[type](el, { pointerId: 1 });
  Object.defineProperty(ev, "clientX", { value: clientX });
  fireEvent(el, ev);
};

const bins = (): HistogramBin[] =>
  Array.from({ length: 20 }, (_, i) => ({ from: i * 0.05, to: (i + 1) * 0.05, count: i + 1 }));

describe("BarChart", () => {
  const items = [
    { key: "a", label: "Alfa", value: 3 },
    { key: "b", label: "Beta", value: 10 },
    { key: "c", label: "Gamma", value: 5 },
  ];

  it("ordina dal valore piu alto e mostra i valori", () => {
    render(<BarChart items={items} emptyText="vuoto" ariaLabel="g" />);
    const order = screen.getAllByRole("img").map((g) => g.getAttribute("aria-label"));
    expect(order).toEqual(["Beta: 10", "Gamma: 5", "Alfa: 3"]);
  });

  it("rispetta sorted=false", () => {
    render(<BarChart items={items} sorted={false} emptyText="vuoto" ariaLabel="g" />);
    expect(screen.getAllByRole("img")[0]).toHaveAttribute("aria-label", "Alfa: 3");
  });

  it("clic, Invio e Spazio selezionano", () => {
    const onSelect = vi.fn();
    render(<BarChart items={items} onSelect={onSelect} emptyText="vuoto" ariaLabel="g" />);
    const bar = screen.getByTestId("bar-b");
    expect(bar).toHaveAttribute("tabindex", "0");
    fireEvent.click(bar);
    fireEvent.keyDown(screen.getByTestId("bar-a"), { key: "Enter" });
    fireEvent.keyDown(screen.getByTestId("bar-c"), { key: " " });
    expect(onSelect.mock.calls.map((c) => c[0])).toEqual(["b", "a", "c"]);
  });

  it("mostra il tooltip al passaggio del puntatore", () => {
    render(<BarChart items={items} emptyText="vuoto" ariaLabel="g" />);
    fireEvent.pointerEnter(screen.getByTestId("bar-b"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Beta: 10");
    fireEvent.pointerLeave(screen.getByTestId("bar-b"));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("stato vuoto", () => {
    render(<BarChart items={[]} emptyText="Nessun dato" ariaLabel="g" />);
    expect(screen.getByRole("status")).toHaveTextContent("Nessun dato");
  });
});

describe("Histogram", () => {
  const mockRect = () => {
    const svg = screen.getByTestId("histogram");
    svg.getBoundingClientRect = () => ({
      left: 0,
      top: 0,
      right: 480,
      bottom: 170,
      width: 480,
      height: 170,
      x: 0,
      y: 0,
      toJSON() {},
    });
    return svg;
  };
  // larghezza 480, LEFT 36 -> bin da 22.2px

  it("mostra soglia e tooltip del bin", () => {
    render(
      <Histogram
        bins={bins()}
        threshold={0.3}
        thresholdLabel="Soglia 0,30"
        emptyText="v"
        ariaLabel="h"
      />,
    );
    expect(screen.getByTestId("histogram-threshold")).toHaveTextContent("Soglia 0,30");
    fireEvent.pointerEnter(screen.getByTestId("bin-5"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("0.25–0.30: 6");
  });

  it("selezione col trascinamento", () => {
    const onRangeChange = vi.fn();
    render(<Histogram bins={bins()} onRangeChange={onRangeChange} emptyText="v" ariaLabel="h" />);
    const svg = mockRect();
    pointer(svg, "pointerDown", 36 + 22.2 * 5 + 2);
    pointer(svg, "pointerMove", 36 + 22.2 * 6 + 2);
    pointer(svg, "pointerUp", 36 + 22.2 * 6 + 2);
    const r = onRangeChange.mock.lastCall![0] as [number, number];
    expect(r[0]).toBeCloseTo(0.25);
    expect(r[1]).toBeCloseTo(0.35);
    expect(screen.getByTestId("histogram-selection")).toBeInTheDocument();
  });

  it("selezione da tastiera con Maiusc+Frecce, Esc la toglie", () => {
    const onRangeChange = vi.fn();
    render(<Histogram bins={bins()} onRangeChange={onRangeChange} emptyText="v" ariaLabel="h" />);
    const svg = screen.getByTestId("histogram");
    fireEvent.keyDown(svg, { key: "ArrowRight" });
    fireEvent.keyDown(svg, { key: "ArrowRight" }); // cursore sul bin 2
    fireEvent.keyDown(svg, { key: "ArrowRight", shiftKey: true });
    const r = onRangeChange.mock.lastCall![0] as [number, number];
    expect(r[0]).toBeCloseTo(0.1);
    expect(r[1]).toBeCloseTo(0.2);
    fireEvent.keyDown(svg, { key: "Escape" });
    expect(onRangeChange).toHaveBeenLastCalledWith(null);
    expect(screen.queryByTestId("histogram-selection")).toBeNull();
  });

  it("da tastiera: tooltip e annuncio del bin sotto il cursore", () => {
    render(<Histogram bins={bins()} emptyText="v" ariaLabel="h" />);
    const svg = screen.getByTestId("histogram");
    fireEvent.focus(svg);
    expect(screen.getByRole("tooltip")).toHaveTextContent("0.00–0.05: 1");
    fireEvent.keyDown(svg, { key: "ArrowRight" });
    expect(screen.getByRole("tooltip")).toHaveTextContent("0.05–0.10: 2");
    expect(screen.getByTestId("histogram-live")).toHaveTextContent("0.05–0.10: 2");
    fireEvent.blur(svg);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("selezione a due passi con Invio", () => {
    const onRangeChange = vi.fn();
    render(<Histogram bins={bins()} onRangeChange={onRangeChange} emptyText="v" ariaLabel="h" />);
    const svg = screen.getByTestId("histogram");
    fireEvent.keyDown(svg, { key: "Enter" }); // ancora sul bin 0
    fireEvent.keyDown(svg, { key: "ArrowRight" });
    fireEvent.keyDown(svg, { key: "ArrowRight" });
    fireEvent.keyDown(svg, { key: "Enter" });
    const r = onRangeChange.mock.lastCall![0] as [number, number];
    expect(r[0]).toBeCloseTo(0);
    expect(r[1]).toBeCloseTo(0.15);
  });

  it("stato vuoto", () => {
    render(<Histogram bins={[]} emptyText="Nessuna confidenza" ariaLabel="h" />);
    expect(screen.getByRole("status")).toHaveTextContent("Nessuna confidenza");
  });
});

describe("StackedBar", () => {
  const segs = [
    { key: "current", label: "Aggiornate", value: 70, tone: "current" as const },
    { key: "stale", label: "Da aggiornare", value: 0, tone: "stale" as const },
    { key: "never", label: "Mai analizzate", value: 30, tone: "never" as const },
  ];

  it("nasconde i segmenti a zero ma li lascia in legenda", () => {
    render(<StackedBar segments={segs} emptyText="v" ariaLabel="s" />);
    expect(screen.queryByTestId("segment-stale")).toBeNull();
    expect(screen.getByTestId("legend-stale")).toHaveTextContent("Da aggiornare: 0");
  });

  it("clic su segmento e legenda", () => {
    const onSelect = vi.fn();
    render(<StackedBar segments={segs} onSelect={onSelect} emptyText="v" ariaLabel="s" />);
    fireEvent.click(screen.getByTestId("segment-never"));
    fireEvent.click(screen.getByTestId("legend-current"));
    fireEvent.keyDown(screen.getByTestId("segment-current"), { key: "Enter" });
    expect(onSelect.mock.calls.map((c) => c[0])).toEqual(["never", "current", "current"]);
  });

  it("stato vuoto con somma zero", () => {
    render(
      <StackedBar
        segments={segs.map((s) => ({ ...s, value: 0 }))}
        emptyText="Niente"
        ariaLabel="s"
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Niente");
  });
});

describe("ColumnSeries", () => {
  const items = Array.from({ length: 36 }, (_, i) => ({
    key: `m${i}`,
    label: `M${i}`,
    total: 10 + i,
    analyzed: i,
  }));

  it("dirada le etichette e mantiene la colonna senza data", () => {
    const { container } = render(
      <ColumnSeries
        items={items}
        noDate={{ key: "nodate", label: "Senza data", total: 4, analyzed: 1 }}
        emptyText="v"
        ariaLabel="c"
        totalLabel="Totale"
        analyzedLabel="Analizzate"
      />,
    );
    const labels = container.querySelectorAll("svg g.pc-chart-mark text");
    expect(labels.length).toBeLessThan(37);
    expect(screen.getByText("Senza data")).toBeInTheDocument();
    expect(screen.getByTestId("column-nodate")).toBeInTheDocument();
  });

  it("tooltip con totale e analizzate, selezione", () => {
    const onSelect = vi.fn();
    render(
      <ColumnSeries
        items={items}
        onSelect={onSelect}
        emptyText="v"
        ariaLabel="c"
        totalLabel="Totale"
        analyzedLabel="Analizzate"
      />,
    );
    fireEvent.pointerEnter(screen.getByTestId("column-m3"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Totale: 13");
    expect(screen.getByRole("tooltip")).toHaveTextContent("Analizzate: 3");
    fireEvent.click(screen.getByTestId("column-m3"));
    expect(onSelect).toHaveBeenCalledWith("m3");
  });

  it("legenda di totali e analizzate", () => {
    render(
      <ColumnSeries
        items={items}
        emptyText="v"
        ariaLabel="c"
        totalLabel="Totale"
        analyzedLabel="Analizzate"
      />,
    );
    const legend = document.querySelector(".pc-chart-legend")!;
    expect(legend).toHaveTextContent("Totale");
    expect(legend).toHaveTextContent("Analizzate");
  });

  it("stato vuoto", () => {
    render(
      <ColumnSeries
        items={[]}
        emptyText="Nessun mese"
        ariaLabel="c"
        totalLabel="T"
        analyzedLabel="A"
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Nessun mese");
  });
});
