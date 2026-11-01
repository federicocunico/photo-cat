import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { PeoplePage } from "./PeoplePage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const all = [
  { uid: "g", name: "Giulia Bianchi", shortName: "Giulia", score: 0.81, person: null },
  { uid: "u", name: "Giuseppe Verdi", shortName: "Giuseppe", score: null, person: null },
  { uid: "r", name: "Rita Rossi", shortName: "Rita", score: 0.2, person: null },
];

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listPeople.mockResolvedValue([
    { id: 12, name: null, identityUid: null, photos: 3, faces: 3, cover: null },
  ] as never);
  commandsMock.personFaces.mockResolvedValue([] as never);
  // Il backend cerca su nome e alias ("Nonna" è un alias di Rita Rossi).
  commandsMock.suggestIdentities.mockImplementation(async (_p: number, q: string) =>
    all.filter((s) =>
      q.toLowerCase() === "nonna" ? s.uid === "r" : s.name.toLowerCase().includes(q.toLowerCase()),
    ),
  );
});
afterEach(cleanup);

async function openUnnamed() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeoplePage />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByText(/senza nome/i));
  return screen.findByRole("combobox", { name: "Chi è?" });
}

it("suggests identities by name or alias, ordered by similarity", async () => {
  const input = await openUnnamed();
  fireEvent.change(input, { target: { value: "Giu" } });
  await screen.findByRole("option", { name: /Giulia Bianchi/ });
  const options = within(screen.getByRole("listbox")).getAllByRole("option");
  expect(options.map((o) => o.textContent)).toEqual([
    "Giulia Bianchi0.81",
    "Giuseppe Verdi",
    'Crea l\'identità "Giu"',
  ]);
  expect(commandsMock.suggestIdentities).toHaveBeenCalledWith(12, "Giu");
  fireEvent.change(input, { target: { value: "Nonna" } });
  await waitFor(() =>
    expect(within(screen.getByRole("listbox")).getAllByRole("option")[0]).toHaveTextContent(
      "Rita Rossi",
    ),
  );
});

it("links an unnamed group to the chosen identity", async () => {
  const input = await openUnnamed();
  fireEvent.change(input, { target: { value: "Giu" } });
  fireEvent.mouseDown(await screen.findByRole("option", { name: /Giulia Bianchi/ }));
  await waitFor(() => expect(commandsMock.linkPersonIdentity).toHaveBeenCalledWith(12, "g"));
});

it("creates the identity when none matches, then links it", async () => {
  commandsMock.saveIdentity.mockResolvedValueOnce("new");
  const input = await openUnnamed();
  fireEvent.change(input, { target: { value: "Marco Russo" } });
  await screen.findByRole("option", { name: 'Crea l\'identità "Marco Russo"' });
  fireEvent.keyDown(input, { key: "Enter" });
  await waitFor(() =>
    expect(commandsMock.saveIdentity).toHaveBeenCalledWith(null, {
      name: "Marco Russo",
      shortName: null,
      group: null,
      threshold: null,
      aliases: [],
    }),
  );
  await waitFor(() => expect(commandsMock.linkPersonIdentity).toHaveBeenCalledWith(12, "new"));
});
