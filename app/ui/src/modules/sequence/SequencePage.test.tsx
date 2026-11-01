import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import "../../test/layoutMock";
import type { SequenceItem, SequenceOverview } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { parseShift, shiftText, useSequence } from "../../store/sequence";
import { SequencePage } from "./SequencePage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const XT5 = 1;
const IPHONE = 2;

const item = (id: number, time: string, group: number, session: number): SequenceItem => ({
  id,
  name: `IMG_${id}.JPG`,
  thumb: null,
  instantUtc: id * 1000,
  local: `2026-08-19 ${time}`,
  localOffsetMin: -360,
  presumed: false,
  groupId: group,
  device: group === XT5 ? "Fujifilm X-T5" : "Apple iPhone 17 Pro Max",
  session,
  tags: id === 1 ? ["Paesaggi"] : [],
});

const items = [
  item(1, "10:41:50", XT5, 0),
  item(2, "10:43:00", IPHONE, 0),
  item(3, "18:30:05", IPHONE, 1),
  item(4, "18:31:00", XT5, 1),
];

const overview = (): SequenceOverview => ({
  sessions: [
    {
      index: 0,
      startLocal: "2026-08-19 10:41:50",
      endLocal: "2026-08-19 10:43:00",
      startUtc: 1,
      endUtc: 2,
      place: "Moab",
      count: 2,
    },
    {
      index: 1,
      startLocal: "2026-08-19 18:30:05",
      endLocal: "2026-08-19 18:31:00",
      startUtc: 3,
      endUtc: 4,
      place: null,
      count: 2,
    },
  ],
  groups: [
    {
      id: XT5,
      device: "Fujifilm X-T5",
      declaredOffsetMin: 120,
      displayTz: null,
      displayOffsetMin: null,
      shiftS: 0,
      count: 64,
    },
    {
      id: IPHONE,
      device: "Apple iPhone 17 Pro Max",
      declaredOffsetMin: -420,
      displayTz: null,
      displayOffsetMin: null,
      shiftS: 0,
      count: 310,
    },
  ],
  undated: [{ id: 9, name: "ricevuta.jpg", thumb: null, fileDate: "2026-10-01 09:00:00" }],
  suggestions: [
    {
      group: XT5,
      device: "Fujifilm X-T5",
      tz: "America/Phoenix",
      offsetMin: -420,
      currentOffsetMin: 120,
      count: 64,
    },
  ],
  total: 4,
  gapMin: 120,
});

function renderPage() {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <SequencePage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useSequence.setState({
    scope: { scope: "library" },
    selected: [],
    focused: null,
    view: "sequence",
  });
  commandsMock.sequenceOverview.mockResolvedValue(overview());
  commandsMock.sequencePage.mockResolvedValue({ items, next: null });
  commandsMock.listCatalogs.mockResolvedValue([]);
});
afterEach(cleanup);

it("parses and prints clock shifts", () => {
  expect(parseShift("+2 min 10 s")).toBe(130);
  expect(parseShift("-0:02:10")).toBe(-130);
  expect(parseShift("145")).toBe(145);
  expect(parseShift("+2 minuti")).toBeNull();
  expect(shiftText(145)).toBe("+2 min 25 s");
  expect(shiftText(-3600)).toBe("-1 h");
});

it("shows sessions with device strip and active-set tag", async () => {
  renderPage();
  expect(await screen.findByText(/2026-08-19 · 10:41–10:43/)).toBeInTheDocument();
  expect(screen.getByText("Moab · 2 foto")).toBeInTheDocument();
  expect(within(screen.getByTestId("seq-1")).getByLabelText("Tag Paesaggi")).toBeInTheDocument();
  expect(screen.getByTestId("seq-1")).toHaveTextContent("10:41:50");
  expect(screen.getByTestId("seq-1")).toHaveTextContent("Fujifilm X-T5");
});

it("applies a clock shift after showing the preview", async () => {
  commandsMock.previewGroupClock.mockResolvedValue([
    { mediaId: 1, before: "2026-08-19 10:41:50", after: "2026-08-19 10:44:00" },
  ]);
  renderPage();
  const xt5 = await screen.findByRole("group", { name: /Fujifilm X-T5/ });
  fireEvent.change(within(xt5).getByLabelText("Scostamento"), { target: { value: "+2 min 10 s" } });
  await waitFor(() =>
    expect(commandsMock.previewGroupClock).toHaveBeenLastCalledWith(XT5, null, null, 130),
  );
  expect(await screen.findByTestId(`preview-${XT5}`)).toHaveTextContent("10:41:50 → 10:44:00");
  expect(commandsMock.setGroupShift).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Applica (1 gruppo)" }));
  await waitFor(() => expect(commandsMock.setGroupShift).toHaveBeenCalledWith(XT5, 130));
  expect(commandsMock.setGroupDisplay).not.toHaveBeenCalled();
});

it("accepts the GPS time zone suggestion", async () => {
  renderPage();
  const banner = await screen.findByRole("alert");
  expect(banner).toHaveTextContent("64 scatti dichiarano +02:00");
  expect(banner).toHaveTextContent("-07:00");
  fireEvent.click(within(banner).getByRole("button", { name: "Applica -07:00" }));
  await waitFor(() =>
    expect(commandsMock.setGroupDisplay).toHaveBeenCalledWith(XT5, "America/Phoenix", null),
  );
});

it("selects the whole session of the focused photo with A", async () => {
  commandsMock.sequenceSessionIds.mockResolvedValue([3, 4]);
  renderPage();
  fireEvent.click(await screen.findByTestId("seq-3"));
  expect(useSequence.getState().selected).toEqual([3]);
  fireEvent.keyDown(window, { key: "a" });
  await waitFor(() => expect(useSequence.getState().selected).toEqual([3, 4]));
  expect(commandsMock.sequenceSessionIds).toHaveBeenCalledWith({ scope: "library" }, 1);
});

it("places an undated photo from the keyboard menu", async () => {
  renderPage();
  fireEvent.click(await screen.findByTestId("seq-3"));
  fireEvent.click(screen.getByRole("button", { name: "Colloca dopo…" }));
  const dialog = screen.getByRole("dialog", { name: "Colloca dopo…" });
  // Il target proposto è la foto con il fuoco.
  expect(within(dialog).getByLabelText("Foto dopo cui collocarla")).toHaveValue("3");
  fireEvent.click(within(dialog).getByRole("button", { name: "Colloca" }));
  await waitFor(() => expect(commandsMock.placeUndated).toHaveBeenCalledWith(9, 3));
});

it("places an undated photo by dragging it after a photo", async () => {
  renderPage();
  const target = await screen.findByTestId("seq-2");
  const data = new Map<string, string>();
  const dataTransfer = {
    types: ["application/x-photocat-undated"],
    setData: (k: string, v: string) => data.set(k, v),
    getData: (k: string) => data.get(k) ?? "",
  };
  fireEvent.dragStart(screen.getByTestId("undated-9"), { dataTransfer });
  fireEvent.dragOver(target, { dataTransfer });
  fireEvent.drop(target, { dataTransfer });
  await waitFor(() => expect(commandsMock.placeUndated).toHaveBeenCalledWith(9, 2));
});

it("aligns two selected photos and stages the proposed shift", async () => {
  commandsMock.proposeAlignment.mockResolvedValue({
    group: XT5,
    currentShiftS: 0,
    proposedShiftS: 145,
    deltaS: 145,
  });
  renderPage();
  fireEvent.click(await screen.findByTestId("seq-3"));
  fireEvent.click(screen.getByTestId("seq-4"), { ctrlKey: true });
  fireEvent.click(screen.getByRole("button", { name: "Allinea le due foto selezionate" }));
  await waitFor(() => expect(commandsMock.proposeAlignment).toHaveBeenCalledWith(3, 4));
  expect(await screen.findByRole("status")).toHaveTextContent("+2 min 25 s");
  const xt5 = screen.getByRole("group", { name: /Fujifilm X-T5/ });
  expect(within(xt5).getByLabelText("Scostamento")).toHaveValue("+2 min 25 s");
});
