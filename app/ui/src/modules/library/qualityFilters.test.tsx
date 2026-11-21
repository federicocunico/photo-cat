import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render as rtlRender, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { emptyFilter, useLibrary } from "../../store/library";
import { DateFacet } from "./DateFacet";
import { RatingFacet } from "./RatingFacet";
import { FacetList } from "./FacetList";
import { activeChips } from "./chips";
import { clearIn, selectedOf, toggleIn } from "./facets";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const render = (ui: React.ReactElement) =>
  rtlRender(<QueryClientProvider client={new QueryClient()}>{ui}</QueryClientProvider>);

const t = i18n.getFixedT("it");
const list = (values: Array<[string, number]>) => ({
  total: values.length,
  values: values.map(([value, count]) => ({ value, count })),
});

beforeEach(() => {
  useLibrary.setState({ filter: emptyFilter() });
  commandsMock.libraryFacetValues.mockResolvedValue({ total: 0, values: [] } as never);
});

it("chip rimovibili per valutazione, colore e anno senza mese", () => {
  const f = {
    ...emptyFilter(),
    rating: { op: "at_least" as const, value: 4 },
    colorTrait: ["sepia" as const, null],
    dateFrom: "2005-01-01",
    dateTo: "2005-12-31",
    dateNoMonth: true,
  };
  const chips = activeChips(f, t as never);
  expect(chips.map((c) => c.name)).toEqual([
    "2005: anno senza mese",
    "Valutazione: almeno 4 stelle",
    "Colore: Seppia",
    "Colore: Non indicato",
  ]);
  expect(chips[0].clear).toEqual({ dateFrom: null, dateTo: null, dateNoMonth: false });
  expect(chips[1].clear).toEqual({ rating: null });
  expect(chips[2].clear).toEqual({ colorTrait: [null] });
  expect(chips[3].clear).toEqual({ colorTrait: ["sepia"] });
});

it("selezione di valutazione e colore nel filtro", () => {
  const f = emptyFilter();
  expect(toggleIn({ kind: "rating" }, f, "4")).toEqual({ rating: { op: "equal", value: 4 } });
  const g = { ...f, rating: { op: "at_least" as const, value: 4 } };
  expect(selectedOf({ kind: "rating" }, g)).toEqual(["4"]);
  expect(toggleIn({ kind: "rating" }, g, "2")).toEqual({ rating: { op: "at_least", value: 2 } });
  expect(toggleIn({ kind: "rating" }, g, "4")).toEqual({ rating: null });
  expect(clearIn({ kind: "rating" }, g)).toEqual({ rating: null });
  expect(toggleIn({ kind: "color" }, f, "none")).toEqual({ colorTrait: [null] });
  const h = { ...f, colorTrait: [null, "bw" as const] };
  expect(selectedOf({ kind: "color" }, h)).toEqual(["none", "bw"]);
  expect(toggleIn({ kind: "color" }, h, "bw")).toEqual({ colorTrait: [null] });
});

it("facet Valutazione: scelta del valore e del confronto", () => {
  render(
    <RatingFacet
      list={list([
        ["5", 1],
        ["4", 2],
        ["3", 0],
        ["2", 0],
        ["1", 0],
        ["0", 7],
      ])}
    />,
  );
  fireEvent.click(screen.getByLabelText(/4 stelle/));
  expect(useLibrary.getState().filter.rating).toEqual({ op: "equal", value: 4 });
  fireEvent.change(screen.getByLabelText("Confronto della valutazione"), {
    target: { value: "at_least" },
  });
  expect(useLibrary.getState().filter.rating).toEqual({ op: "at_least", value: 4 });
});

it("facet Colore: voci con nomi tradotti, Non indicato incluso", () => {
  render(
    <FacetList
      facetKey={{ kind: "color" }}
      list={list([
        ["bw", 3],
        ["none", 5],
      ])}
    />,
  );
  fireEvent.click(screen.getByLabelText(/Non indicato/));
  expect(useLibrary.getState().filter.colorTrait).toEqual([null]);
  fireEvent.click(screen.getByLabelText(/Bianco e nero/));
  expect(useLibrary.getState().filter.colorTrait).toEqual([null, "bw"]);
});

it("nodo Anno senza mese nel DateFacet", () => {
  useLibrary.setState({
    filter: { ...emptyFilter(), dateFrom: "2005-01-01", dateTo: "2005-12-31" },
  });
  render(
    <DateFacet
      years={list([["2005", 4]])}
      months={list([
        ["2005-00", 2],
        ["2005-06", 2],
      ])}
      days={null}
    />,
  );
  fireEvent.click(screen.getByLabelText(/Anno senza mese/));
  expect(useLibrary.getState().filter).toMatchObject({
    dateNoMonth: true,
    dateFrom: "2005-01-01",
    dateTo: "2005-12-31",
  });
  fireEvent.click(screen.getByLabelText(/Anno senza mese/));
  expect(useLibrary.getState().filter.dateNoMonth).toBe(false);
});
