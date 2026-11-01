import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, it, expect, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
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
  commandsMock.personFaces.mockResolvedValue([
    { id: 10, mediaId: 1, bbox: [0.1, 0.1, 0.2, 0.2], quality: 0.9, thumb: null, personId: 1 },
  ] as never);
});
afterEach(cleanup);
it("opens a cluster, offers the identity field and sends a persistent refusal", async () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByText("Alice"));
  expect(screen.getByRole("combobox", { name: "Chi è?" })).toBeInTheDocument();
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
