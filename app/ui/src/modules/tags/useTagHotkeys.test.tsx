import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { TagOverview } from "../../bindings";
import { commandsMock, emit, ipcMockModule, job } from "../../test/ipcMock";
import { useLibrary } from "../../store/library";
import { useLiveEvents } from "../../store/live";
import { useTagHotkeys } from "./useTagHotkeys";
vi.mock("../../lib/ipc", () => ipcMockModule(true));

const overview: TagOverview = {
  tags: [],
  sets: [
    {
      id: 1,
      name: "Base",
      exclusive: false,
      isBase: true,
      exportKeywords: false,
      tags: [{ key: 1, name: "Preferite" }],
      tagged: 0,
    },
    {
      id: 2,
      name: "Viaggio USA 2026",
      exclusive: true,
      isBase: false,
      exportKeywords: false,
      tags: [
        { key: 1, name: "Paesaggi" },
        { key: 2, name: "Città" },
        { key: 3, name: "Tramonti" },
        { key: 4, name: "Persone" },
        { key: 5, name: "Cibo" },
      ],
      tagged: 0,
    },
  ],
  activeSet: 2,
  palette: [],
};

function Harness() {
  useLiveEvents();
  useTagHotkeys(() => useLibrary.getState().selected);
  return <input type="search" aria-label="Ricerca" />;
}

let client: QueryClient;
function renderHarness() {
  client = new QueryClient();
  render(
    <QueryClientProvider client={client}>
      <Harness />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.tagOverview.mockResolvedValue(overview);
  commandsMock.startBulk.mockResolvedValue(41);
  commandsMock.listJobs.mockResolvedValue([]);
  commandsMock.getResources.mockResolvedValue(undefined);
  useLibrary.setState({ selected: [10, 11, 12, 13, 14, 15] });
});
afterEach(cleanup);

it("key 3 tags the whole selection with the third tag of the active set", async () => {
  renderHarness();
  await waitFor(() => expect(commandsMock.tagOverview).toHaveBeenCalled());
  await waitFor(() => {
    fireEvent.keyDown(window, { key: "3" });
    expect(commandsMock.startBulk).toHaveBeenCalled();
  });
  expect(commandsMock.startBulk).toHaveBeenCalledTimes(1);
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    [10, 11, 12, 13, 14, 15],
    { action: "add_tag", name: "Tramonti" },
    false,
  );
  // A job concluso le viste si aggiornano senza ricaricare.
  const refresh = vi.spyOn(client, "invalidateQueries");
  act(() => emit("job", job({ id: 41, kind: "bulk", state: "completed" })));
  await waitFor(() => expect(refresh).toHaveBeenCalledWith({ queryKey: ["library"] }));
});

it("selection of 2,340 unseen results is sent whole", async () => {
  const all = Array.from({ length: 2340 }, (_, i) => i + 1);
  useLibrary.setState({ selected: all });
  renderHarness();
  await waitFor(() => {
    fireEvent.keyDown(window, { key: "2" });
    expect(commandsMock.startBulk).toHaveBeenCalled();
  });
  const call = commandsMock.startBulk.mock.calls[0] as unknown as [number[], unknown];
  expect(call[0]).toHaveLength(2340);
  expect(call[1]).toEqual({ action: "add_tag", name: "Città" });
});

it("key 0 removes the active set's tags", async () => {
  renderHarness();
  await waitFor(() => {
    fireEvent.keyDown(window, { key: "0" });
    expect(commandsMock.startBulk).toHaveBeenCalled();
  });
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    [10, 11, 12, 13, 14, 15],
    { action: "remove_tags", names: ["Paesaggi", "Città", "Tramonti", "Persone", "Cibo"] },
    false,
  );
});

it("does nothing while typing in the search box, for free keys or without selection", async () => {
  renderHarness();
  await waitFor(() => expect(commandsMock.tagOverview).toHaveBeenCalled());
  // Attende che il set attivo sia caricato: un tasto valido funziona.
  await waitFor(() => {
    fireEvent.keyDown(window, { key: "1" });
    expect(commandsMock.startBulk).toHaveBeenCalledTimes(1);
  });
  const search = screen.getByLabelText("Ricerca");
  search.focus();
  fireEvent.keyDown(search, { key: "3" });
  fireEvent.keyDown(window, { key: "9" }); // tasto senza tag nel set
  fireEvent.keyDown(window, { key: "3", ctrlKey: true });
  useLibrary.setState({ selected: [] });
  fireEvent.keyDown(window, { key: "3" });
  expect(commandsMock.startBulk).toHaveBeenCalledTimes(1);
});
