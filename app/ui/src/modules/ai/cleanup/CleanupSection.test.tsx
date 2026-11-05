import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../../i18n";
import { commandsMock, ipcMockModule } from "../../../test/ipcMock";
import { useLive } from "../../../store/live";
import { useUndo } from "../../../store/undo";
import { useToast } from "../../../store/toasts";
import { CleanupSection } from "../CleanupSection";
import { useCleanupPreset } from "../results/cleanupPreset";

vi.mock("../../../lib/ipc", () => ipcMockModule(true));

const mocks = commandsMock as unknown as Record<string, ReturnType<typeof vi.fn>>;

const taxonomy = {
  version: 1,
  groups: [
    {
      id: "scene",
      label: "Scena",
      threshold: null,
      categories: [
        { id: "nature", label: "Natura", enabled: true, prompts: [] },
        { id: "city", label: "Città", enabled: true, prompts: [] },
      ],
    },
  ],
};

const wrap = () =>
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <CleanupSection />
    </QueryClientProvider>,
  );

beforeEach(() => {
  vi.clearAllMocks();
  useCleanupPreset.getState().clear();
  useLive.setState({ jobs: {} });
  mocks.getTaxonomy = vi.fn(async () => taxonomy);
  mocks.listCatalogs = vi.fn(async () => [
    {
      catalog: { id: 7, name: "Vacanze", kind: "manual", rules: null, createdAt: 0 },
      items: 3,
      excluded: 0,
    },
    {
      catalog: {
        id: 9,
        name: "Cestino",
        kind: "manual",
        rules: null,
        createdAt: 0,
        system: "trash",
      },
      items: 0,
      excluded: 0,
    },
  ]);
  mocks.librarySelection = vi.fn(async () => ({ maxId: 100, count: 50 }));
  mocks.aiRejectPreview = vi.fn(async () => 12);
  mocks.aiRejectLabels = vi.fn(async () => 5);
  mocks.undoBulk = vi.fn(async () => null);
  mocks.aiLastIsRejection = vi.fn(async () => true);
  mocks.aiUndoRejection = vi.fn(async () => null);
});
afterEach(cleanup);

it("precompila dal preset e lo svuota; anteprima con debounce", async () => {
  useCleanupPreset
    .getState()
    .set({ grp: "scene", value: "nature", maxScore: 0.35, scopeCatalogId: 7 });
  wrap();
  expect(await screen.findByText(/12 foto perderanno l'etichetta «Natura»/)).toBeInTheDocument();
  expect(useCleanupPreset.getState().grp).toBeNull();
  expect(mocks.aiRejectPreview).toHaveBeenCalledTimes(1);
  const [g, v, max, target] = mocks.aiRejectPreview.mock.calls[0];
  expect([g, v, max]).toEqual(["scene", "nature", 0.35]);
  expect(target).toMatchObject({ kind: "filter", maxId: 100, filter: { catalogId: 7 } });
  expect(screen.queryByRole("option", { name: /Cestino/ })).toBeNull();
});

it("conferma disabilitata con 0 foto", async () => {
  mocks.aiRejectPreview = vi.fn(async () => 0);
  useCleanupPreset.getState().set({ grp: "scene", value: "city" });
  wrap();
  expect(await screen.findByText(/Nessuna foto ha l'etichetta «Città»/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Togli l'etichetta…/ })).toBeDisabled();
});

it("conferma: invia il rifiuto e, a job finito, mostra l'avviso con annullamento", async () => {
  useCleanupPreset
    .getState()
    .set({ grp: "scene", value: "nature", maxScore: 0.35, scopeCatalogId: 7 });
  wrap();
  await screen.findByText(/12 foto perderanno/);
  fireEvent.click(screen.getByRole("button", { name: /Togli l'etichetta…/ }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent(/non la riassegneranno/);
  fireEvent.click(screen.getAllByRole("button", { name: "Togli l'etichetta" }).pop()!);
  await waitFor(() => expect(mocks.aiRejectLabels).toHaveBeenCalledTimes(1));
  const [g, v, max, target] = mocks.aiRejectLabels.mock.calls[0];
  expect([g, v, max]).toEqual(["scene", "nature", 0.35]);
  expect(target).toMatchObject({
    kind: "filter",
    maxId: 100,
    except: [],
    filter: { catalogId: 7 },
  });

  expect(useUndo.getState().last).toBeNull();
  act(() =>
    useLive.setState({
      jobs: { 5: { id: 5, state: "completed", failed: 0, phases: [] } as never },
    }),
  );
  await waitFor(() => expect(useUndo.getState().last).not.toBeNull());
  await act(async () => {
    await useUndo.getState().last!.run();
  });
  expect(mocks.aiUndoRejection).toHaveBeenCalled();
  expect(mocks.undoBulk).not.toHaveBeenCalled();
});

it("l'avviso con annullamento compare anche se si lascia la sezione prima della fine", async () => {
  useUndo.setState({ last: null } as never);
  useCleanupPreset.getState().set({ grp: "scene", value: "nature" });
  const { unmount } = wrap();
  await screen.findByText(/12 foto perderanno/);
  fireEvent.click(screen.getByRole("button", { name: /Togli l'etichetta…/ }));
  await screen.findByRole("alertdialog");
  fireEvent.click(screen.getAllByRole("button", { name: "Togli l'etichetta" }).pop()!);
  await waitFor(() => expect(mocks.aiRejectLabels).toHaveBeenCalledTimes(1));
  // Job in corso: niente secondo invio.
  await waitFor(() =>
    expect(screen.getByRole("button", { name: /Togli l'etichetta…/ })).toBeDisabled(),
  );
  unmount();
  act(() =>
    useLive.setState({
      jobs: { 5: { id: 5, state: "completed", failed: 0, phases: [] } as never },
    }),
  );
  await waitFor(() => expect(useUndo.getState().last).not.toBeNull());
});

it("pulizia senza modifiche: avviso neutro, niente annullamento di altre operazioni", async () => {
  useUndo.setState({ last: null } as never);
  mocks.aiLastIsRejection = vi.fn(async () => false);
  useCleanupPreset.getState().set({ grp: "scene", value: "nature" });
  wrap();
  await screen.findByText(/12 foto perderanno/);
  fireEvent.click(screen.getByRole("button", { name: /Togli l'etichetta…/ }));
  await screen.findByRole("alertdialog");
  fireEvent.click(screen.getAllByRole("button", { name: "Togli l'etichetta" }).pop()!);
  await waitFor(() => expect(mocks.aiRejectLabels).toHaveBeenCalledTimes(1));
  act(() =>
    useLive.setState({
      jobs: { 5: { id: 5, state: "completed", failed: 0, phases: [] } as never },
    }),
  );
  await waitFor(() => expect(mocks.aiLastIsRejection).toHaveBeenCalled());
  await waitFor(() =>
    expect(
      useToast.getState().notices.some((n) => n.message === "Nessuna etichetta da togliere"),
    ).toBe(true),
  );
  expect(useUndo.getState().last).toBeNull();
  expect(mocks.undoBulk).not.toHaveBeenCalled();
});
