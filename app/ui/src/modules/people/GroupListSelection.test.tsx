// Elenco di Persone sul modello di selezione comune (unified-selection, task 4.2).
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { GroupRow, IdentityRow } from "../../lib/ipc";
import { useGridShortcuts } from "../../commands/gridShortcuts";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { runSelectionContract } from "../../test/selectionContract";
import { GroupList, type GroupListTarget } from "./GroupList";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const group = (id: number): GroupRow => ({
  id,
  name: null,
  identityUid: null,
  section: "review",
  faces: 10 - id,
  goodFaces: 10 - id,
  discardedFaces: 0,
  photos: 10 - id,
  cover: null,
  ignoredAt: null,
  newFaces: 0,
  nameHint: null,
  mergeHint: null,
});
const IDS = [1, 2, 3, 4, 5, 6, 7, 8];

/** Come il modulo Persone: selezione e gruppo aperto vivono nel genitore. */
const opened: { current: GroupListTarget } = { current: null };
function Host() {
  const [target, setTarget] = useState<GroupListTarget>(null);
  const [ids, setIds] = useState<number[]>([]);
  useGridShortcuts();
  opened.current = target;
  return (
    <GroupList
      section="review"
      onSectionChange={() => {}}
      target={target}
      selectedIds={ids}
      onSelect={(next) => {
        setTarget(next.target);
        setIds(next.selectedIds);
      }}
      onSelectedIdsChange={setIds}
    />
  );
}

async function mount() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <div style={{ height: 600, width: 320, display: "flex" }}>
        <Host />
      </div>
    </QueryClientProvider>,
  );
  // Le voci dei gruppi (non le <option> del filtro per numero di foto).
  await waitFor(() => expect(document.querySelectorAll("[data-group-id]")).toHaveLength(8));
}
const list = () => screen.getByRole("listbox", { name: "Elenco persone e gruppi" });
const rows = () => within(list()).getAllByRole("option");

beforeEach(() => {
  vi.clearAllMocks();
  opened.current = null;
  commandsMock.peopleSections.mockResolvedValue({
    review: 8,
    people: 0,
    ignored: 0,
    singles: 0,
    discarded: 0,
  } as never);
  commandsMock.listGroups.mockResolvedValue({
    items: IDS.map(group),
    next: null,
    total: IDS.length,
  } as never);
  commandsMock.groupSelectionIds.mockResolvedValue(IDS as never);
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

runSelectionContract({
  name: "elenco dei gruppi di Persone",
  layout: "list",
  mount,
  container: list,
  items: rows,
});

describe("elenco di Persone da tastiera", () => {
  it("Estendere con Maiusc: dal secondo gruppo, tre volte Maiusc+giù", async () => {
    await mount();
    fireEvent.click(rows()[1]!);
    for (let i = 0; i < 3; i++) fireEvent.keyDown(list(), { key: "ArrowDown", shiftKey: true });
    const selected = rows()
      .filter((r) => r.getAttribute("aria-selected") === "true")
      .map((r) => r.dataset.groupId);
    expect(selected).toEqual(["2", "3", "4", "5"]);
    expect(list()).toHaveAttribute("aria-activedescendant", "group-opt-5");
  });

  it("le frecce selezionano subito e aprono il gruppo raggiunto dopo una breve pausa", async () => {
    await mount();
    fireEvent.click(rows()[0]!);
    expect(opened.current).toEqual({ kind: "group", id: 1 });
    vi.useFakeTimers();
    try {
      fireEvent.keyDown(list(), { key: "ArrowDown" });
      fireEvent.keyDown(list(), { key: "ArrowDown" });
      // La selezione segue subito la freccia; l'apertura aspetta 150 ms di quiete.
      expect(rows()[2]).toHaveAttribute("aria-selected", "true");
      expect(opened.current).toEqual({ kind: "group", id: 1 });
      act(() => {
        vi.advanceTimersByTime(160);
      });
      expect(opened.current).toEqual({ kind: "group", id: 3 });
    } finally {
      vi.useRealTimers();
    }
  });

  it("Ctrl+frecce spostano solo il focus e Invio apre il gruppo con il focus", async () => {
    await mount();
    fireEvent.click(rows()[0]!);
    fireEvent.keyDown(list(), { key: "ArrowDown", ctrlKey: true });
    fireEvent.keyDown(list(), { key: "ArrowDown", ctrlKey: true });
    expect(rows()[0]).toHaveAttribute("aria-selected", "true");
    expect(rows()[2]).toHaveAttribute("aria-selected", "false");
    fireEvent.keyDown(list(), { key: "Enter" });
    await waitFor(() => expect(opened.current).toEqual({ kind: "group", id: 3 }));
  });
});

// Stesso contratto sull'elenco delle persone (identità: id testuali).
const UIDS = ["a", "b", "c", "d", "e", "f", "g", "h"];
const identity = (uid: string): IdentityRow => ({
  uid,
  name: `Persona ${uid.toUpperCase()}`,
  shortName: uid,
  group: null,
  threshold: null,
  aliases: [],
  faces: 0,
  createdAt: 0,
  person: null,
  photos: 0,
});
function PeopleHost() {
  const [target, setTarget] = useState<GroupListTarget>(null);
  const [ids, setIds] = useState<number[]>([]);
  useGridShortcuts();
  return (
    <GroupList
      section="people"
      onSectionChange={() => {}}
      target={target}
      selectedIds={ids}
      onSelect={(next) => {
        setTarget(next.target);
        setIds(next.selectedIds);
      }}
      onSelectedIdsChange={setIds}
    />
  );
}
const identities = () => screen.getByRole("listbox", { name: "Identità" });

runSelectionContract({
  name: "elenco delle persone",
  layout: "list",
  mount: async () => {
    commandsMock.listIdentities.mockResolvedValue({
      identities: UIDS.map(identity),
      groups: [],
      storePath: "",
      defaultThreshold: 0.363,
    } as never);
    commandsMock.identitySelectionUids.mockResolvedValue(UIDS);
    render(
      <QueryClientProvider client={new QueryClient()}>
        <div style={{ height: 600, width: 320, display: "flex" }}>
          <PeopleHost />
        </div>
      </QueryClientProvider>,
    );
    await screen.findByText("Persona H");
  },
  container: identities,
  items: () => within(identities()).getAllByRole("option"),
});

describe("menu contestuale dell'elenco", () => {
  it("su un gruppo selezionato agisce su tutta la selezione e ne dice il numero", async () => {
    await mount();
    fireEvent.click(rows()[0]!);
    fireEvent.click(rows()[2]!, { shiftKey: true });
    fireEvent.contextMenu(rows()[1]!);
    expect(
      await screen.findByRole("menuitem", { name: "Assegna 3 gruppi a…" }),
    ).toBeInTheDocument();
  });

  it("su un gruppo non selezionato seleziona solo quello", async () => {
    await mount();
    fireEvent.click(rows()[0]!);
    fireEvent.click(rows()[2]!, { shiftKey: true });
    fireEvent.contextMenu(rows()[5]!);
    expect(
      await screen.findByRole("menuitem", { name: "Assegna il gruppo a…" }),
    ).toBeInTheDocument();
    expect(
      rows()
        .filter((r) => r.getAttribute("aria-selected") === "true")
        .map((r) => r.dataset.groupId),
    ).toEqual(["6"]);
  });
});

it("in Persone la ricerca per iniziale non è attiva: le lettere restano alle scorciatoie", async () => {
  await mount();
  fireEvent.click(rows()[0]!);
  fireEvent.keyDown(list(), { key: "p" });
  fireEvent.keyDown(list(), { key: "e" });
  expect(
    rows()
      .filter((r) => r.getAttribute("aria-selected") === "true")
      .map((r) => r.dataset.groupId),
  ).toEqual(["1"]);
  expect(list()).toHaveAttribute("aria-activedescendant", "group-opt-1");
});
