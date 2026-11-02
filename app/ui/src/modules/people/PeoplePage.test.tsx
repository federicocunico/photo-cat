import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, it, expect, vi } from "vitest";
import { act } from "@testing-library/react";
import "../../i18n";
import { commandsMock, emit, ipcMockModule } from "../../test/ipcMock";
import { PeoplePage } from "./PeoplePage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));
beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listPeople.mockResolvedValue([
    { id: 1, name: "Alice", photos: 2, faces: 2, cover: null },
    { id: 2, name: "Bob", photos: 1, faces: 1, cover: null },
  ] as never);
  commandsMock.personFaces.mockResolvedValue({
    items: [
      { id: 10, mediaId: 1, bbox: [0.1, 0.1, 0.2, 0.2], quality: 0.9, thumb: null, personId: 1 },
    ],
    next: null,
  } as never);
});
afterEach(cleanup);
it("Elenco vuoto: only shows the empty message after loading has finished", async () => {
  let finish!: (value: never[]) => void;
  commandsMock.listPeople.mockImplementationOnce(
    () =>
      new Promise((r) => {
        finish = r;
      }),
  );
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  expect(screen.queryByText(/nessun volto/i)).not.toBeInTheDocument();
  expect(screen.getByRole("status", { name: "Caricamento…" })).toBeInTheDocument();
  finish([]);
  expect(await screen.findByText(/nessun volto/i)).toBeInTheDocument();
});
it("opens a cluster, offers the identity field and sends a persistent refusal", async () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByText("Alice"));
  expect(await screen.findByRole("combobox", { name: "Chi è?" })).toBeInTheDocument();
  fireEvent.click(await screen.findByText("Non è questa persona"));
  await waitFor(() =>
    expect(commandsMock.correctPerson).toHaveBeenCalledWith({
      action: "reject",
      person: 1,
      face: 10,
    }),
  );
});
it("shows the photo count per person and labels unnamed clusters", async () => {
  commandsMock.listPeople.mockResolvedValue([
    { id: 1, name: "Alice", photos: 20, faces: 23, cover: null },
    { id: 7, name: null, photos: 1, faces: 1, cover: null },
  ] as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  const alice = (await screen.findByText("Alice")).closest("button")!;
  expect(alice).toHaveTextContent("20 foto");
  const unnamed = screen.getByText(/senza nome/i).closest("button")!;
  expect(unnamed).toHaveTextContent("1 foto");
  expect(unnamed).not.toHaveTextContent("null");
});
it("non interroga a intervalli: persone e volti si aggiornano su faces://changed", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  try {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <PeoplePage />
      </QueryClientProvider>,
    );
    fireEvent.click(await screen.findByText("Alice"));
    await waitFor(() => expect(commandsMock.personFaces).toHaveBeenCalledTimes(1));
    expect(commandsMock.listPeople).toHaveBeenCalledTimes(1);
    // Nessuna richiesta periodica, anche dopo un minuto.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(commandsMock.listPeople).toHaveBeenCalledTimes(1);
    expect(commandsMock.personFaces).toHaveBeenCalledTimes(1);
    // L'evento del backend invalida le query.
    act(() => emit("facesChanged", undefined));
    await waitFor(() => expect(commandsMock.listPeople).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(commandsMock.personFaces).toHaveBeenCalledTimes(2));
  } finally {
    vi.useRealTimers();
  }
});
it('i volti arrivano a pagine: "Mostra altri" chiede la pagina successiva', async () => {
  commandsMock.personFaces
    .mockResolvedValueOnce({
      items: [
        { id: 10, mediaId: 1, bbox: [0.1, 0.1, 0.2, 0.2], quality: 0.9, thumb: null, personId: 1 },
      ],
      next: { quality: 0.9, id: 10 },
    } as never)
    .mockResolvedValueOnce({
      items: [
        { id: 11, mediaId: 2, bbox: [0.1, 0.1, 0.2, 0.2], quality: 0.5, thumb: null, personId: 1 },
      ],
      next: null,
    } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByText("Alice"));
  fireEvent.click(await screen.findByText("Mostra altri volti"));
  await waitFor(() =>
    expect(commandsMock.personFaces).toHaveBeenLastCalledWith(1, { quality: 0.9, id: 10 }, 200),
  );
  expect(await screen.findByLabelText("#11")).toBeInTheDocument();
  expect(screen.queryByText("Mostra altri volti")).not.toBeInTheDocument();
});
