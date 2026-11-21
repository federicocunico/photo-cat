import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { TagOverview } from "../../bindings";
import { commandsMock, emit, ipcMockModule, job } from "../../test/ipcMock";
import { idsSelection, selCount, selTarget, useLibrary } from "../../store/library";
import { useLiveEvents } from "../../store/live";
import { useToast } from "../../store/toasts";
import { useTagHotkeys } from "../tags/useTagHotkeys";
import { useRatingHotkeys } from "./useRatingHotkeys";
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
      tags: [
        { key: 2, name: "Città" },
        { key: 5, name: "Cibo" },
      ],
      tagged: 0,
    },
  ],
  activeSet: 1,
  palette: [],
};

function Harness() {
  useLiveEvents();
  const selection = () => {
    const s = useLibrary.getState().selection;
    return { target: selTarget(s), count: selCount(s) };
  };
  useTagHotkeys(selection);
  useRatingHotkeys(selection);
  return <input type="search" aria-label="Ricerca" />;
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.tagOverview.mockResolvedValue(overview);
  commandsMock.startBulk.mockResolvedValue(51);
  commandsMock.listJobs.mockResolvedValue([]);
  commandsMock.getResources.mockResolvedValue(undefined);
  useLibrary.setState({ selection: idsSelection([1, 2, 3]) });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Harness />
    </QueryClientProvider>,
  );
});
afterEach(cleanup);

const ids = { kind: "ids", ids: [1, 2, 3] };

it("Alt+5 imposta la valutazione 5 su 3 foto, senza toccare i tag", async () => {
  await waitFor(() => expect(commandsMock.tagOverview).toHaveBeenCalled());
  fireEvent.keyDown(window, { key: "5", altKey: true });
  await waitFor(() => expect(commandsMock.startBulk).toHaveBeenCalledTimes(1));
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    ids,
    { action: "set_rating", rating: 5 },
    false,
  );
});

it("Alt+0 toglie la valutazione", async () => {
  fireEvent.keyDown(window, { key: "0", altKey: true });
  await waitFor(() => expect(commandsMock.startBulk).toHaveBeenCalledTimes(1));
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    ids,
    { action: "set_rating", rating: null },
    false,
  );
});

it("il tasto 2 senza Alt assegna ancora il tag e non cambia la valutazione", async () => {
  await waitFor(() => {
    fireEvent.keyDown(window, { key: "2" });
    expect(commandsMock.startBulk).toHaveBeenCalled();
  });
  expect(commandsMock.startBulk).toHaveBeenCalledTimes(1);
  expect(commandsMock.startBulk).toHaveBeenCalledWith(
    ids,
    { action: "add_tag", name: "Città" },
    false,
  );
});

it("ignora Alt+numero nei campi di testo e senza selezione", async () => {
  const search = screen.getByLabelText("Ricerca");
  search.focus();
  fireEvent.keyDown(search, { key: "3", altKey: true });
  useLibrary.setState({ selection: idsSelection() });
  fireEvent.keyDown(window, { key: "3", altKey: true });
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
});

it("a operazione finita compare l'avviso con Annulla", async () => {
  fireEvent.keyDown(window, { key: "4", altKey: true });
  await waitFor(() => expect(commandsMock.startBulk).toHaveBeenCalled());
  act(() => emit("job", job({ id: 51, kind: "bulk", state: "completed" })));
  await waitFor(() =>
    expect(useToast.getState().notices).toContainEqual(
      expect.objectContaining({
        message: "Valutazione impostata su 3 foto",
        action: expect.objectContaining({ label: expect.stringMatching(/Annulla/) }),
      }),
    ),
  );
});
