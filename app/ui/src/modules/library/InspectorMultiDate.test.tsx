// Data di scatto nell'Inspector con più foto (bulk-date-inspector, issue #68).
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { emptyFilter, idsSelection, selectAllOf, useLibrary } from "../../store/library";
import { Inspector } from "./Inspector";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import type { MediaDates } from "../../lib/ipc";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const FILE_2008: MediaDates = {
  capturedAt: "2008-05-10 12:00:00",
  capturedSource: "exif_original",
  modifiedAt: null,
};
const manual = (at: string, precision: "day" | "month" | "year"): MediaDates => ({
  ...FILE_2008,
  capturedSource: "manual",
  manualAt: at,
  manualPrecision: precision,
});

function detail(id: number, dates: MediaDates | null) {
  return {
    item: {
      id,
      kind: "photo",
      sourceId: 1,
      path: `D:\\Scansioni\\Praga_${id}.tif`,
      name: `Praga_${id}.tif`,
      ext: "tif",
      size: 1000,
      mtime: 0,
      quickHash: null,
      contentHash: null,
      addedAt: 0,
      updatedAt: 0,
      thumb: null,
      capturedAt: null,
      width: 100,
      height: 100,
      status: "ok",
      faces: 0,
      analysis: "processed",
    },
    rating: 0,
    colorTrait: null,
    tech: null,
    dates,
    exif: null,
    location: null,
    labels: [],
    peopleCount: 0,
    people: [],
    faces: [],
    tags: [],
    catalogs: [],
    duplicates: [],
  };
}

/** Dettagli per id: `datesOf(id)` dà la data di ciascuna foto. */
function withDates(datesOf: (id: number) => MediaDates | null) {
  commandsMock.mediaDetail.mockImplementation((async (id: number) =>
    detail(id, datesOf(id))) as never);
}

/** Inspector su `ids`; risolve quando i dettagli di tutte le foto sono arrivati. */
async function renderInspector(ids: number[]) {
  useLibrary.setState({ focused: ids[ids.length - 1]!, selection: idsSelection(ids) });
  const qc = new QueryClient();
  render(
    <QueryClientProvider client={qc}>
      <Inspector />
    </QueryClientProvider>,
  );
  // Prima dei dettagli la data non è nota e la riga mostra "Valori diversi".
  await waitFor(() =>
    expect(ids.every((id) => qc.getQueryData(["media-detail", id]) != null)).toBe(true),
  );
}

const editDate = async (shown: RegExp) =>
  fireEvent.click(await screen.findByRole("button", { name: shown }));

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.getTaxonomy.mockResolvedValue({ version: 1, groups: [] } as never);
  commandsMock.listCatalogs.mockResolvedValue([] as never);
});
afterEach(checkedCleanup(A11Y_KNOWN.library));

it("Data completa su più foto dall'Inspector: data comune del file, poi 14 agosto 1976", async () => {
  withDates(() => FILE_2008);
  await renderInspector([1, 2, 3]);
  expect(await screen.findByText("3 foto selezionate")).toBeInTheDocument();
  // Stessa data del file per tutte: la riga la mostra; nessuna data manuale, nessun ripristino.
  await editDate(/^Acquisizione: 10\/05\/08/);
  expect(screen.queryByRole("button", { name: "Ripristina data del file" })).toBeNull();
  // Parte su "giorno" con i campi vuoti: "Imposta data" disattivato.
  expect(screen.getByRole("radio", { name: "Giorno" })).toBeChecked();
  expect(screen.getByRole("spinbutton", { name: "Anno" })).toHaveValue(null);
  expect(screen.getByRole("spinbutton", { name: "Giorno" })).toHaveValue(null);
  expect(screen.getByRole("combobox", { name: "Mese" })).toHaveValue("");
  const set = screen.getByRole("button", { name: "Imposta data" });
  expect(set).toBeDisabled();
  fireEvent.change(screen.getByRole("spinbutton", { name: "Giorno" }), { target: { value: "14" } });
  fireEvent.change(screen.getByRole("combobox", { name: "Mese" }), { target: { value: "08" } });
  expect(set).toBeDisabled();
  fireEvent.change(screen.getByRole("spinbutton", { name: "Anno" }), { target: { value: "1976" } });
  fireEvent.click(set);
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [1, 2, 3] },
      { action: "set_manual_date", value: "1976-08-14", precision: "day" },
      false,
    ),
  );
});

it("Solo anno su più foto", async () => {
  withDates(() => FILE_2008);
  await renderInspector([1, 2]);
  await editDate(/^Acquisizione:/);
  fireEvent.click(screen.getByRole("radio", { name: "Anno" }));
  fireEvent.change(screen.getByRole("spinbutton", { name: "Anno" }), { target: { value: "1976" } });
  fireEvent.click(screen.getByRole("button", { name: "Imposta data" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [1, 2] },
      { action: "set_manual_date", value: "1976", precision: "year" },
      false,
    ),
  );
});

it("Date diverse nella selezione: Valori diversi, editor su giorno con campi vuoti", async () => {
  withDates((id) => (id === 2 ? { ...FILE_2008, capturedAt: "2009-01-01 08:00:00" } : FILE_2008));
  await renderInspector([1, 2, 3]);
  await editDate(/^Acquisizione: Valori diversi/);
  expect(screen.getByRole("radio", { name: "Giorno" })).toBeChecked();
  expect(screen.getByRole("spinbutton", { name: "Anno" })).toHaveValue(null);
  // Nessuna data manuale tra quelle note: niente ripristino.
  expect(screen.queryByRole("button", { name: "Ripristina data del file" })).toBeNull();
});

it("Data manuale comune: l'editor parte da giugno 1984 con precisione mese", async () => {
  withDates(() => manual("1984-06-01 00:00:00", "month"));
  await renderInspector([1, 2, 3, 4, 5]);
  await editDate(/giugno 1984 \(impostata a mano\)/);
  expect(screen.getByRole("radio", { name: "Mese" })).toBeChecked();
  expect(screen.getByRole("combobox", { name: "Mese" })).toHaveValue("06");
  expect(screen.getByRole("spinbutton", { name: "Anno" })).toHaveValue(1984);
});

it("Ripristino su più foto con date manuali diverse", async () => {
  withDates((id) =>
    id === 1 ? manual("1962-01-01 00:00:00", "year") : manual("1984-06-01 00:00:00", "month"),
  );
  await renderInspector([1, 2]);
  await editDate(/^Acquisizione: Valori diversi/);
  fireEvent.click(screen.getByRole("button", { name: "Ripristina data del file" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      { kind: "ids", ids: [1, 2] },
      { action: "clear_manual_date" },
      false,
    ),
  );
});

it("Selezione per filtro: Valori diversi, data e ripristino su tutto il filtro", async () => {
  const filter = emptyFilter();
  useLibrary.setState({
    focused: 1,
    selection: selectAllOf(filter, { count: 120, maxId: 500 }),
  });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Inspector />
    </QueryClientProvider>,
  );
  await editDate(/^Acquisizione: Valori diversi/);
  // Valori non noti: il ripristino resta disponibile.
  expect(screen.getByRole("button", { name: "Ripristina data del file" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("radio", { name: "Anno" }));
  fireEvent.change(screen.getByRole("spinbutton", { name: "Anno" }), { target: { value: "1976" } });
  fireEvent.click(screen.getByRole("button", { name: "Imposta data" }));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      expect.objectContaining({ kind: "filter", maxId: 500 }),
      { action: "set_manual_date", value: "1976", precision: "year" },
      false,
    ),
  );
  expect(commandsMock.mediaDetail).not.toHaveBeenCalled();
});
