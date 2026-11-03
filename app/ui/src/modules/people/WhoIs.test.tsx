import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { GroupRow } from "../../lib/ipc";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { PeopleModule } from "./PeopleModule";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const all = [
  { uid: "g", name: "Giulia Bianchi", shortName: "Giulia", score: 0.81, person: null },
  { uid: "u", name: "Giuseppe Verdi", shortName: "Giuseppe", score: null, person: null },
  { uid: "r", name: "Rita Rossi", shortName: "Rita", score: 0.2, person: null },
];

function row(partial: Partial<GroupRow> & { id: number }): GroupRow {
  return {
    name: null,
    identityUid: null,
    section: "review",
    faces: 3,
    goodFaces: 3,
    discardedFaces: 0,
    photos: 3,
    cover: null,
    ignoredAt: null,
    newFaces: 0,
    nameHint: null,
    mergeHint: null,
    ...partial,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLayout.setState({
    module: "people",
    peopleSection: "review",
    theme: "dark",
    language: "it",
  });
  commandsMock.peopleSections.mockResolvedValue({
    review: 1,
    people: 0,
    ignored: 0,
    singles: 0,
    discarded: 0,
  } as never);
  commandsMock.listGroups.mockResolvedValue({
    items: [row({ id: 12, faces: 3, photos: 3 })],
    next: null,
    total: 1,
  } as never);
  commandsMock.groupFaces.mockResolvedValue({ items: [], next: null, total: 0 } as never);
  commandsMock.suggestIdentities.mockImplementation(async (_p: number, q: string) =>
    all.filter((s) =>
      q.toLowerCase() === "nonna" ? s.uid === "r" : s.name.toLowerCase().includes(q.toLowerCase()),
    ),
  );
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

async function openUnnamed() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <PeopleModule />
    </QueryClientProvider>,
  );
  fireEvent.click(await screen.findByText(/Gruppo di 3 volti/i));
  return screen.findByRole("combobox", { name: "Chi è?" });
}

it("suggests identities by name or alias, ordered by similarity", async () => {
  const input = await openUnnamed();
  fireEvent.change(input, { target: { value: "Giu" } });
  await screen.findByRole("option", { name: /Giulia Bianchi/ });
  const options = within(screen.getByRole("listbox", { name: /Identità proposte/i })).getAllByRole(
    "option",
  );
  expect(options.map((o) => o.textContent)).toEqual([
    "Giulia Bianchi0.81",
    "Giuseppe Verdi",
    'Crea l\'identità "Giu"',
  ]);
  expect(commandsMock.suggestIdentities).toHaveBeenCalledWith(12, "Giu");
  fireEvent.change(input, { target: { value: "Nonna" } });
  await waitFor(() =>
    expect(
      within(screen.getByRole("listbox", { name: /Identità proposte/i })).getAllByRole("option")[0],
    ).toHaveTextContent("Rita Rossi"),
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
