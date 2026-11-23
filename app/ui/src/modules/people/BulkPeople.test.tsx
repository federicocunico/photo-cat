import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useUndo } from "../../store/undo";
import { useCommandStore } from "../../commands/registry";
import { useGridShortcuts } from "../../commands/gridShortcuts";
import { ToastHost } from "../../components/feedback/ToastHost";
import type { FaceView, GroupRow, IdentityRow } from "../../lib/ipc";
import { PeopleModule } from "./PeopleModule";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

function Shell() {
  useGridShortcuts();
  return null;
}

function renderModule() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <Shell />
      <PeopleModule />
      <ToastHost />
    </QueryClientProvider>,
  );
}

function identity(uid: string, name: string, extra: Partial<IdentityRow> = {}): IdentityRow {
  return {
    uid,
    name,
    shortName: name,
    group: null,
    threshold: null,
    aliases: [],
    faces: 0,
    createdAt: 0,
    person: null,
    photos: 0,
    ...extra,
  };
}

function group(id: number, extra: Partial<GroupRow> = {}): GroupRow {
  return {
    id,
    name: null,
    identityUid: null,
    section: "review",
    faces: 1,
    goodFaces: 1,
    discardedFaces: 0,
    photos: 1,
    cover: null,
    ignoredAt: null,
    newFaces: 0,
    nameHint: null,
    mergeHint: null,
    ...extra,
  };
}

function face(id: number): FaceView {
  return {
    id,
    mediaId: id,
    bbox: [0.1, 0.1, 0.3, 0.3],
    quality: 0.9,
    qualityScore: 0.8,
    sharpness: 0.7,
    frontality: 0.9,
    sizePx: 64,
    discardReason: null,
    year: 2024,
    thumb: null,
    personId: null,
  };
}

const view = (identities: IdentityRow[]) =>
  ({ identities, groups: [], storePath: "", defaultThreshold: 0.363 }) as never;

const SMALL = Array.from({ length: 340 }, (_, i) => identity(`s${i}`, `Sconosciuto ${i}`));
const SMALL_UIDS = SMALL.map((i) => i.uid);
const DEST = identity("dest", "Sconosciuti", { person: 5, photos: 900, faces: 3 });

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useUndo.setState({ last: null, busy: false });
  useLayout.setState({ module: "people", peopleSection: "people", theme: "dark", language: "it" });
  commandsMock.peopleSections.mockResolvedValue({
    review: 420,
    people: 341,
    ignored: 0,
    singles: 0,
    discarded: 0,
  } as never);
  commandsMock.listIdentities.mockImplementation((async (_q: string | null, max: number | null) =>
    view(max == null ? [DEST, ...SMALL] : SMALL)) as never);
  commandsMock.identitySelectionUids.mockResolvedValue(SMALL_UIDS);
  commandsMock.mergeIdentitiesPreview.mockResolvedValue({
    identities: 340,
    photos: 512,
    referenceFaces: 40,
    withoutPhotos: 0,
    names: ["Sconosciuto 0", "Sconosciuto 1", "Sconosciuto 2", "Sconosciuto 3", "Sconosciuto 4"],
  });
  commandsMock.mergeIdentitiesMany.mockImplementation(
    (async (from: string[]) => from.length) as never,
  );
  commandsMock.groupFaces.mockResolvedValue({
    items: [],
    next: null,
    total: 0,
    goodCount: 0,
    discardedCount: 0,
  } as never);
});
afterEach(() => {
  checkedCleanup(A11Y_KNOWN.people)();
  useCommandStore.setState({ commands: [] });
});

const pickFilter = (value: string) =>
  fireEvent.change(screen.getByRole("combobox", { name: "Mostra solo con poche foto" }), {
    target: { value },
  });

async function openMergeDialog() {
  fireEvent.click(await screen.findByRole("button", { name: "Unisci in…" }));
  return screen.findByRole("dialog");
}

async function chooseDestination(dialog: HTMLElement, search: string, option: string) {
  const box = await within(dialog).findByRole("combobox", { name: "Cerca una persona" });
  fireEvent.change(box, { target: { value: search } });
  fireEvent.click(await within(dialog).findByRole("option", { name: new RegExp(option) }));
}

vi.setConfig({ testTimeout: 20000 });

describe("Persone: unione di più identità", () => {
  it("molte identità piccole: filtro, Seleziona tutto, conferma con i conteggi e unione", async () => {
    renderModule();
    await screen.findByText("Sconosciuto 0");
    pickFilter("2");
    await waitFor(() => expect(commandsMock.listIdentities).toHaveBeenCalledWith(null, 2));
    await waitFor(() => expect(screen.queryByText("Sconosciuti")).not.toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "Seleziona tutto" }));
    await screen.findByText("340 selezionati");
    expect(commandsMock.identitySelectionUids).toHaveBeenCalledWith(null, 2);

    const dialog = await openMergeDialog();
    await chooseDestination(dialog, "Sconos", "^Sconosciuti");
    await waitFor(() =>
      expect(commandsMock.mergeIdentitiesPreview).toHaveBeenCalledWith(SMALL_UIDS, "dest"),
    );
    expect(await within(dialog).findByText("Unire 340 identità in «Sconosciuti»?")).toBeVisible();
    expect(
      within(dialog).getByText(
        /^512 foto in questa libreria e 40 volti di riferimento passeranno a «Sconosciuti»\. Le identità selezionate verranno eliminate: i loro nomi resteranno come alias di «Sconosciuti».*L'operazione non si può annullare\.$/,
      ),
    ).toBeVisible();
    // Chi sparisce, non solo quante.
    expect(
      within(dialog).getByText(
        "Verranno unite: Sconosciuto 0, Sconosciuto 1, Sconosciuto 2, Sconosciuto 3, Sconosciuto 4 e altre 335.",
      ),
    ).toBeVisible();
    expect(within(dialog).queryByText(/non hanno foto in questa libreria/)).not.toBeInTheDocument();
    // Senza il pulsante esplicito non succede nulla.
    expect(commandsMock.mergeIdentitiesMany).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole("button", { name: "Unisci in «Sconosciuti»" }));
    await waitFor(() =>
      expect(commandsMock.mergeIdentitiesMany).toHaveBeenCalledWith(SMALL_UIDS, "dest"),
    );
    expect(await screen.findByText("340 identità unite in «Sconosciuti»")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.queryByText("340 selezionati")).not.toBeInTheDocument();
  });

  it("senza conferma non unisce nulla: Indietro e Annulla non chiamano mergeIdentitiesMany", async () => {
    renderModule();
    await screen.findByText("Sconosciuto 0");
    fireEvent.click(screen.getByText("Sconosciuto 1"));
    const dialog = await openMergeDialog();
    await chooseDestination(dialog, "Sconos", "^Sconosciuti");
    fireEvent.click(await within(dialog).findByRole("button", { name: "Indietro" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "Annulla" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(commandsMock.mergeIdentitiesMany).not.toHaveBeenCalled();
  });

  it("la destinazione può essere una delle selezionate", async () => {
    const trio = [
      identity("a", "G. Bianchi"),
      identity("b", "Giulia B."),
      identity("c", "Giulia Bianchi"),
    ];
    commandsMock.listIdentities.mockResolvedValue(view(trio));
    commandsMock.mergeIdentitiesPreview.mockResolvedValue({
      identities: 2,
      photos: 10,
      referenceFaces: 1,
      withoutPhotos: 0,
      names: ["G. Bianchi", "Giulia B."],
    });
    renderModule();
    fireEvent.click(await screen.findByText("G. Bianchi"));
    fireEvent.click(screen.getByText("Giulia B."), { ctrlKey: true });
    fireEvent.click(screen.getByText("Giulia Bianchi"), { ctrlKey: true });
    await screen.findByText("3 selezionati");

    const dialog = await openMergeDialog();
    await chooseDestination(dialog, "Giulia Bianchi", "Giulia Bianchi");
    await within(dialog).findByText("Unire 2 identità in «Giulia Bianchi»?");
    expect(commandsMock.mergeIdentitiesPreview).toHaveBeenCalledWith(["a", "b", "c"], "c");
    fireEvent.click(within(dialog).getByRole("button", { name: "Unisci in «Giulia Bianchi»" }));
    await waitFor(() =>
      expect(commandsMock.mergeIdentitiesMany).toHaveBeenCalledWith(["a", "b", "c"], "c"),
    );
  });

  it("Maiusc+clic seleziona un intervallo, Ctrl+A e Esc seguono il focus dell'elenco", async () => {
    renderModule();
    fireEvent.click(await screen.findByText("Sconosciuto 1"));
    fireEvent.click(screen.getByText("Sconosciuto 4"), { shiftKey: true });
    await screen.findByText("4 selezionati");

    screen.getByRole("listbox", { name: "Identità" }).focus();

    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("340 selezionati");
    expect(commandsMock.identitySelectionUids).toHaveBeenCalledWith(null, null);

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText("340 selezionati")).not.toBeInTheDocument());
  });

  it("un clic semplice apre ancora la scheda dell'identità", async () => {
    commandsMock.listIdentities.mockResolvedValue(view([identity("x", "Marco Rossi")]));
    renderModule();
    fireEvent.click(await screen.findByText("Marco Rossi"));
    expect((await screen.findAllByDisplayValue("Marco Rossi")).length).toBeGreaterThan(0);
  });

  it("un errore dell'unione resta visibile con il motivo e non annuncia successo", async () => {
    commandsMock.mergeIdentitiesMany.mockRejectedValue({ detail: "Archivio bloccato" });
    renderModule();
    await screen.findByText("Sconosciuto 0");
    fireEvent.click(screen.getByText("Sconosciuto 1"));
    const dialog = await openMergeDialog();
    await chooseDestination(dialog, "Sconos", "^Sconosciuti");
    fireEvent.click(await within(dialog).findByRole("button", { name: "Unisci in «Sconosciuti»" }));
    expect(await within(dialog).findByText("Archivio bloccato")).toBeVisible();
    expect(screen.queryByText(/identità unit/)).not.toBeInTheDocument();
  });

  it("avvisa se tra le identità unite ce ne sono senza foto in questa libreria", async () => {
    commandsMock.mergeIdentitiesPreview.mockResolvedValue({
      identities: 3,
      photos: 2,
      referenceFaces: 0,
      withoutPhotos: 2,
      names: ["Sconosciuto 1", "Sconosciuto 2", "Sconosciuto 3"],
    });
    renderModule();
    fireEvent.click(await screen.findByText("Sconosciuto 1"));
    fireEvent.click(screen.getByText("Sconosciuto 3"), { shiftKey: true });
    const dialog = await openMergeDialog();
    await chooseDestination(dialog, "Sconos", "^Sconosciuti");
    expect(
      await within(dialog).findByText(
        /^2 di queste identità non hanno foto in questa libreria: se usi altre librerie/,
      ),
    ).toBeVisible();
    expect(
      within(dialog).getByText("Verranno unite: Sconosciuto 1, Sconosciuto 2, Sconosciuto 3."),
    ).toBeVisible();
  });

  it("cambiare il filtro azzera la selezione: nessuna riga nascosta resta selezionata", async () => {
    renderModule();
    fireEvent.click(await screen.findByText("Sconosciuto 1"));
    fireEvent.click(screen.getByText("Sconosciuto 3"), { shiftKey: true });
    await screen.findByText("3 selezionati");
    pickFilter("2");
    await waitFor(() => expect(screen.queryByText("3 selezionati")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Unisci in…" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("l'elenco è virtualizzato: con 3000 identità ne disegna poche ma Ctrl+A le seleziona tutte", async () => {
    const MANY = Array.from({ length: 3000 }, (_, i) => identity(`m${i}`, `Persona ${i}`));
    commandsMock.listIdentities.mockResolvedValue(view(MANY));
    commandsMock.identitySelectionUids.mockResolvedValue(MANY.map((i) => i.uid));
    renderModule();
    await screen.findByText("Persona 0");
    const list = screen.getByRole("listbox", { name: "Identità" });
    expect(within(list).getAllByRole("option").length).toBeLessThan(100);
    expect(screen.queryByText("Persona 2999")).not.toBeInTheDocument();
    list.focus();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(await screen.findByText("3000 selezionati")).toBeInTheDocument();
  });
});

describe("Da rivedere: assegnazione in blocco", () => {
  const IDS = Array.from({ length: 420 }, (_, i) => i + 1);
  const PIERO = group(9, { name: "Nonno Piero", section: "people", photos: 120, faces: 130 });

  beforeEach(() => {
    useLayout.setState({ peopleSection: "review" });
    commandsMock.listGroups.mockImplementation((async (section: string) =>
      section === "people"
        ? { items: [PIERO], next: null, total: 1 }
        : { items: [group(1), group(2), group(3)], next: null, total: 420 }) as never);
    commandsMock.groupSelectionIds.mockResolvedValue(IDS);
    commandsMock.groupFaces.mockResolvedValue({
      items: [face(1)],
      next: null,
      total: 1,
      goodCount: 1,
      discardedCount: 0,
    } as never);
  });

  /** Le righe dei gruppi (non le voci del filtro per numero di foto, anch'esse "option"). */
  const groupRows = async () =>
    within(await screen.findByRole("listbox", { name: "Elenco persone e gruppi" })).findAllByRole(
      "option",
      { name: /1 foto/ },
    );

  async function selectAllOnes() {
    await groupRows();
    pickFilter("1");
    await waitFor(() =>
      expect(commandsMock.listGroups).toHaveBeenCalledWith(
        "review",
        null,
        "photos_desc",
        null,
        50,
        1,
      ),
    );
    const first = (await groupRows())[0]!;
    first.focus();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("420 selezionati");
    expect(commandsMock.groupSelectionIds).toHaveBeenCalledWith("review", null, 1);
  }

  it("filtro 1 foto, Ctrl+A, Assegna a una persona esistente con avviso e Annulla", async () => {
    renderModule();
    await selectAllOnes();
    commandsMock.assignGroups.mockResolvedValue({
      person: 9,
      created: false,
      groups: 420,
      faces: 435,
      skippedFaces: 0,
    });

    fireEvent.click(screen.getByRole("button", { name: "Assegna a…" }));
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Nonno", "Nonno Piero");
    expect(await within(dialog).findByText("Assegnare 420 gruppi a «Nonno Piero»?")).toBeVisible();
    expect(commandsMock.assignGroups).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: "Assegna a «Nonno Piero»" }));

    await waitFor(() =>
      expect(commandsMock.assignGroups).toHaveBeenCalledWith(IDS, { kind: "person", person: 9 }),
    );
    const notice = await screen.findByText("420 gruppi assegnati a «Nonno Piero» (435 volti)");
    expect(notice).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText("420 selezionati")).not.toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: /^Annulla/ }));
    await waitFor(() => expect(commandsMock.undoPeople).toHaveBeenCalledTimes(1));
  });

  it("nuova persona: saveIdentity e poi assignGroups verso l'identità", async () => {
    commandsMock.saveIdentity.mockResolvedValue("nuova-uid");
    renderModule();
    await selectAllOnes();
    fireEvent.click(screen.getByRole("button", { name: "Assegna a…" }));
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Vicino di casa", "Nuova persona «Vicino di casa»");
    fireEvent.click(
      await within(dialog).findByRole("button", { name: "Assegna a «Vicino di casa»" }),
    );
    await waitFor(() =>
      expect(commandsMock.assignGroups).toHaveBeenCalledWith(IDS, {
        kind: "identity",
        uid: "nuova-uid",
      }),
    );
    expect(commandsMock.saveIdentity).toHaveBeenCalledWith(
      null,
      expect.objectContaining({ name: "Vicino di casa" }),
    );
    expect(commandsMock.saveIdentity.mock.invocationCallOrder[0]).toBeLessThan(
      commandsMock.assignGroups.mock.invocationCallOrder[0]!,
    );
  });

  it("spiega i volti non assegnati perché rifiutati", async () => {
    commandsMock.assignGroups.mockResolvedValue({
      person: 9,
      created: false,
      groups: 418,
      faces: 430,
      skippedFaces: 3,
    });
    renderModule();
    await selectAllOnes();
    fireEvent.click(screen.getByRole("button", { name: "Assegna a…" }));
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Nonno", "Nonno Piero");
    fireEvent.click(await within(dialog).findByRole("button", { name: "Assegna a «Nonno Piero»" }));
    expect(
      await screen.findByText(
        /3 volti erano stati indicati come non di questa persona e restano da rivedere\./,
      ),
    ).toBeInTheDocument();
  });

  it("nuova persona: se l'assegnazione fallisce l'identità appena creata viene eliminata", async () => {
    commandsMock.saveIdentity.mockResolvedValue("nuova-uid");
    commandsMock.assignGroups.mockRejectedValue({ detail: "Libreria bloccata" });
    renderModule();
    await selectAllOnes();
    fireEvent.click(screen.getByRole("button", { name: "Assegna a…" }));
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Vicino di casa", "Nuova persona «Vicino di casa»");
    fireEvent.click(
      await within(dialog).findByRole("button", { name: "Assegna a «Vicino di casa»" }),
    );
    expect(await within(dialog).findByText("Libreria bloccata")).toBeVisible();
    await waitFor(() => expect(commandsMock.deleteIdentity).toHaveBeenCalledWith("nuova-uid"));
  });

  it("cambiare il filtro azzera la selezione dei gruppi", async () => {
    renderModule();
    await selectAllOnes();
    pickFilter("2");
    await waitFor(() => expect(screen.queryByText("420 selezionati")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Assegna a…" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("oltre 10.000 gruppi spiega il limite prima della conferma", async () => {
    const HUGE = Array.from({ length: 10_001 }, (_, i) => i + 1);
    commandsMock.groupSelectionIds.mockResolvedValue(HUGE);
    renderModule();
    const first = (await groupRows())[0]!;
    first.focus();
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("10001 selezionati");
    fireEvent.click(screen.getByRole("button", { name: "Assegna a…" }));
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Nonno", "Nonno Piero");
    expect(await within(dialog).findByText(/al massimo 10\.000 gruppi alla volta/)).toBeVisible();
    expect(within(dialog).queryByRole("button", { name: /^Assegna a «/ })).not.toBeInTheDocument();
    expect(commandsMock.assignGroups).not.toHaveBeenCalled();
  });

  it("Ctrl+A segue la vista su cui si lavora: volti del gruppo aperto o elenco dei gruppi", async () => {
    commandsMock.faceSelection.mockResolvedValue({ count: 1, maxId: 1 });
    renderModule();
    const row = (await groupRows())[0]!;
    fireEvent.pointerDown(row);
    fireEvent.click(row);
    const tile = await screen.findByTestId("face-1");
    // Ultimo clic sui volti: Ctrl+A seleziona i volti, non i gruppi.
    fireEvent.pointerDown(tile);
    fireEvent.click(tile);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await waitFor(() => expect(commandsMock.faceSelection).toHaveBeenCalled());
    expect(commandsMock.groupSelectionIds).not.toHaveBeenCalled();
    // Di nuovo sull'elenco (riquadro attivo con il clic): Ctrl+A seleziona i gruppi.
    fireEvent.mouseDown((await groupRows())[1]!);
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await waitFor(() => expect(commandsMock.groupSelectionIds).toHaveBeenCalled());
  });

  it("senza selezione Assegna a… spiega perché non è disponibile", async () => {
    renderModule();
    await screen.findAllByRole("option", { name: /1 foto/ });
    const button = screen.getByRole("button", { name: "Assegna a…" });
    expect(button).toHaveAttribute("aria-disabled", "true");
  });
});

describe("Singoli: assegnazione in blocco", () => {
  it("Ctrl+A sui volti e Assegna a… usa correctPerson", async () => {
    useLayout.setState({ peopleSection: "review" });
    commandsMock.peopleSections.mockResolvedValue({
      review: 1,
      people: 1,
      ignored: 0,
      singles: 230,
      discarded: 0,
    } as never);
    commandsMock.listGroups.mockImplementation((async (section: string) =>
      section === "people"
        ? { items: [], next: null, total: 0 }
        : { items: [group(1)], next: null, total: 1 }) as never);
    commandsMock.listIdentities.mockResolvedValue(view([identity("sc", "Sconosciuti")]));
    commandsMock.groupFaces.mockResolvedValue({
      items: [face(1), face(2)],
      next: null,
      total: 230,
      goodCount: 230,
      discardedCount: 0,
    } as never);
    const ALL = Array.from({ length: 230 }, (_, i) => i + 1);
    commandsMock.faceSelection.mockResolvedValue({ count: 230, maxId: 230 });
    commandsMock.faceSelectionIds.mockResolvedValue(ALL);

    renderModule();
    fireEvent.click(await screen.findByRole("option", { name: /230 volti non raggruppati/i }));
    await screen.findByTestId("face-1");
    // Un clic su un volto sposta il focus nella griglia: Ctrl+A seleziona tutti i volti.
    fireEvent.click(screen.getByTestId("face-1"));
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findAllByText("230 selezionati");

    fireEvent.click(
      within(screen.getByRole("toolbar", { name: "Azioni sui volti" })).getByRole("button", {
        name: "Assegna a…",
      }),
    );
    const dialog = await screen.findByRole("dialog");
    await chooseDestination(dialog, "Sconos", "^Sconosciuti");
    expect(await within(dialog).findByText("Assegnare 230 volti a «Sconosciuti»?")).toBeVisible();
    fireEvent.click(within(dialog).getByRole("button", { name: "Assegna a «Sconosciuti»" }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "assign_identity",
        identity: "sc",
        faces: ALL,
      }),
    );
    expect(await screen.findByText("230 volti assegnati a «Sconosciuti»")).toBeInTheDocument();
  });
});
