import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, emit, ipcMockModule } from "../../test/ipcMock";
import { migrateLayoutState, useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import type { FaceView, GroupRow, LocatedFace } from "../../lib/ipc";
import { PeopleModule } from "./PeopleModule";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

function row(partial: Partial<GroupRow> & { id: number }): GroupRow {
  return {
    name: null,
    identityUid: null,
    section: "review",
    faces: 10,
    goodFaces: 9,
    discardedFaces: 1,
    photos: 8,
    cover: null,
    ignoredAt: null,
    newFaces: 0,
    nameHint: null,
    mergeHint: null,
    ...partial,
  };
}

function face(partial: Partial<FaceView> & { id: number }): FaceView {
  return {
    mediaId: 1,
    bbox: [0.1, 0.1, 0.3, 0.3],
    quality: 0.9,
    qualityScore: 0.8,
    sharpness: 0.7,
    frontality: 0.9,
    sizePx: 64,
    discardReason: null,
    year: 2024,
    thumb: null,
    personId: 1,
    ...partial,
  };
}

function located(
  partial: Partial<LocatedFace> & Pick<LocatedFace, "faceId" | "section">,
): LocatedFace {
  return {
    mediaId: 7,
    mediaName: "IMG_4821.JPG",
    groupId: 1,
    singles: false,
    discarded: false,
    index: 0,
    ...partial,
  };
}

function renderModule() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <PeopleModule />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useLayout.setState({
    module: "people",
    peopleSection: "review",
    peopleTarget: null,
    theme: "dark",
    language: "it",
  });
  commandsMock.peopleSections.mockResolvedValue({
    review: 38,
    people: 12,
    ignored: 5,
    singles: 412,
    discarded: 18,
  } as never);
  commandsMock.listGroups.mockResolvedValue({
    items: [
      row({
        id: 1,
        faces: 214,
        photos: 168,
        nameHint: {
          uid: "u1",
          name: "Giulia",
          shortName: "Giulia",
          group: null,
          score: 0.71,
        },
      }),
      row({ id: 2, faces: 61, photos: 58 }),
    ],
    next: null,
    total: 2,
  } as never);
  commandsMock.listIdentities.mockResolvedValue({
    identities: [
      {
        uid: "g",
        name: "Giulia",
        shortName: "Giulia",
        group: null,
        aliases: [],
        threshold: null,
        person: 10,
        photos: 400,
        faces: 2,
        createdAt: 0,
      },
    ],
    groups: [],
    storePath: "C:\\app.db",
    defaultThreshold: 0.363,
  } as never);
  commandsMock.groupFaces.mockResolvedValue({
    items: [face({ id: 380, personId: 10 })],
    next: null,
    total: 1,
    goodCount: 1,
    discardedCount: 0,
  } as never);
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

describe("PeopleModule", () => {
  it("mostra le sezioni con i conteggi da peopleSections", async () => {
    renderModule();
    const tabs = await screen.findByRole("tablist");
    const review = within(tabs).getByRole("tab", { name: /Da rivedere/i });
    const people = within(tabs).getByRole("tab", { name: /^Persone/i });
    const ignored = within(tabs).getByRole("tab", { name: /Ignorati/i });
    await waitFor(() => expect(review).toHaveTextContent("38"));
    expect(people).toHaveTextContent("12");
    expect(ignored).toHaveTextContent("5");
    expect(review).toHaveAttribute("aria-selected", "true");

    await waitFor(() => expect(commandsMock.listGroups).toHaveBeenCalled());
    expect(commandsMock.listGroups.mock.calls[0][0]).toBe("review");
    expect(await screen.findByText(/Gruppo di 214 volti/i)).toBeInTheDocument();
    expect(screen.getByText(/168 foto/)).toBeInTheDocument();
    expect(screen.getByText(/412 volti non raggruppati/i)).toBeInTheDocument();
  });

  it("passa alla sezione Persone e richiede listIdentities", async () => {
    commandsMock.listIdentities.mockResolvedValue({
      identities: [],
      groups: [],
      storePath: "C:\\app.db",
      defaultThreshold: 0.363,
    } as never);
    renderModule();
    const tabs = await screen.findByRole("tablist");
    fireEvent.click(within(tabs).getByRole("tab", { name: /^Persone/i }));
    await waitFor(() => expect(commandsMock.listIdentities).toHaveBeenCalled());
    expect(useLayout.getState().peopleSection).toBe("people");
  });
});

describe("Preferenza del modulo non più esistente", () => {
  it("migra identities → people sulla sezione Persone", async () => {
    expect(migrateLayoutState({ module: "identities", leftWidth: 240 })).toMatchObject({
      module: "people",
      peopleSection: "people",
      leftWidth: 240,
    });

    // setState scrive subito su localStorage: va fatto prima del fixture v0.
    useLayout.setState({ module: "library", peopleSection: "review" });
    localStorage.setItem(
      "photo-cat.layout",
      JSON.stringify({
        state: {
          module: "identities",
          leftOpen: true,
          rightOpen: true,
          leftWidth: 240,
          rightWidth: 300,
          theme: "dark",
          language: "it",
        },
        version: 0,
      }),
    );
    await act(async () => {
      await useLayout.persist.rehydrate();
    });
    expect(useLayout.getState().module).toBe("people");
    expect(useLayout.getState().peopleSection).toBe("people");
  });
});

describe("Apertura diretta di una persona", () => {
  it("Dall'Inspector: apre Giulia con il volto evidenziato", async () => {
    commandsMock.locateFace.mockResolvedValue(
      located({
        faceId: 380,
        section: "people",
        groupId: 10,
        index: 379,
      }),
    );
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 10, name: "Giulia", section: "people", faces: 400, photos: 400 })],
      next: null,
      total: 1,
    } as never);
    commandsMock.groupFaces.mockResolvedValue({
      items: [face({ id: 380, personId: 10 })],
      next: null,
      total: 400,
      goodCount: 400,
      discardedCount: 0,
    } as never);

    renderModule();
    act(() => {
      useLayout.getState().openPeople({ face: 380 });
    });

    await waitFor(() =>
      expect(commandsMock.locateFace).toHaveBeenCalledWith({
        kind: "face",
        face_id: 380,
      }),
    );
    await waitFor(() => expect(useLayout.getState().peopleSection).toBe("people"));
    expect(useLayout.getState().peopleTarget).toBeNull();
    expect(await screen.findByTestId("people-arrival")).toHaveTextContent(/IMG_4821\.JPG/);
    expect(screen.getByRole("button", { name: /Torna alla foto/i })).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("face-380")).toHaveAttribute("data-highlighted", "true"),
    );

    fireEvent.click(screen.getByRole("button", { name: /Torna alla foto/i }));
    expect(useLayout.getState().module).toBe("library");
    expect(useLibrary.getState().focused).toBe(7);
  });

  it("Volto non riconosciuto: apre Da rivedere con il volto evidenziato", async () => {
    commandsMock.locateFace.mockResolvedValue(
      located({
        faceId: 12,
        section: "review",
        groupId: 3,
        index: 0,
      }),
    );
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 3, faces: 5, photos: 5 })],
      next: null,
      total: 1,
    } as never);
    commandsMock.groupFaces.mockResolvedValue({
      items: [face({ id: 12, personId: 3 })],
      next: null,
      total: 5,
      goodCount: 5,
      discardedCount: 0,
    } as never);

    renderModule();
    act(() => {
      useLayout.getState().openPeople({ media: 7, person: null });
    });

    await waitFor(() =>
      expect(commandsMock.locateFace).toHaveBeenCalledWith({
        kind: "media",
        media_id: 7,
        person_id: null,
      }),
    );
    await waitFor(() => expect(useLayout.getState().peopleSection).toBe("review"));
    expect(await screen.findByTestId("people-arrival")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("face-12")).toHaveAttribute("data-highlighted", "true"),
    );
  });

  it("Volto ignorato o scartato: apre Ignorati o i volti scartati", async () => {
    commandsMock.locateFace.mockResolvedValue(
      located({
        faceId: 55,
        section: "ignored",
        groupId: 8,
        index: 0,
      }),
    );
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 8, section: "ignored", faces: 20, photos: 18, ignoredAt: 1 })],
      next: null,
      total: 1,
    } as never);
    commandsMock.groupFaces.mockResolvedValue({
      items: [face({ id: 55, personId: 8 })],
      next: null,
      total: 20,
      goodCount: 20,
      discardedCount: 0,
    } as never);

    renderModule();
    act(() => {
      useLayout.getState().openPeople({ face: 55 });
    });

    await waitFor(() => expect(useLayout.getState().peopleSection).toBe("ignored"));
    await waitFor(() =>
      expect(screen.getByTestId("face-55")).toHaveAttribute("data-highlighted", "true"),
    );

    cleanup();
    useLayout.setState({ peopleSection: "review", peopleTarget: null, module: "people" });
    commandsMock.locateFace.mockResolvedValue(
      located({
        faceId: 99,
        section: "review",
        groupId: 3,
        discarded: true,
        index: 1,
      }),
    );
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 3, faces: 10, discardedFaces: 2 })],
      next: null,
      total: 1,
    } as never);
    commandsMock.groupFaces.mockImplementation(async (scope) => {
      if (scope && typeof scope === "object" && "kind" in scope && scope.kind === "discarded") {
        return {
          items: [
            face({ id: 90, discardReason: "blurred" }),
            face({ id: 99, discardReason: "blurred" }),
          ],
          next: null,
          total: 2,
          goodCount: 8,
          discardedCount: 2,
        };
      }
      return {
        items: [face({ id: 1 })],
        next: null,
        total: 8,
        goodCount: 8,
        discardedCount: 2,
      };
    });

    renderModule();
    act(() => {
      useLayout.getState().openPeople({ face: 99 });
    });

    await waitFor(() => expect(useLayout.getState().peopleSection).toBe("review"));
    await waitFor(() =>
      expect(screen.getByTestId("face-99")).toHaveAttribute("data-highlighted", "true"),
    );
    expect(commandsMock.groupFaces).toHaveBeenCalledWith(
      { kind: "discarded", group: 3 },
      "quality_desc",
      null,
      80,
    );
  });
});

describe("Revisione da tastiera", () => {
  it("frecce → N → nome → conferma, poi focus sul gruppo successivo", async () => {
    const first = row({ id: 1, faces: 10, photos: 8 });
    const second = row({ id: 2, faces: 20, photos: 15 });
    commandsMock.listGroups.mockResolvedValue({
      items: [first, second],
      next: null,
      total: 2,
    } as never);
    commandsMock.groupFaces.mockResolvedValue({
      items: [face({ id: 101, personId: 1 })],
      next: null,
      total: 1,
      goodCount: 1,
      discardedCount: 0,
    } as never);
    commandsMock.suggestIdentities.mockResolvedValue([] as never);
    commandsMock.saveIdentity.mockResolvedValue("uid-new" as never);
    commandsMock.linkPersonIdentity.mockImplementation(async (person: number) => person);

    const client = new QueryClient();
    render(
      <QueryClientProvider client={client}>
        <PeopleModule />
      </QueryClientProvider>,
    );

    const list = await screen.findByTestId("group-list-scroll");
    await within(list).findByText(/Gruppo di 10 volti/i);
    fireEvent.keyDown(window, { key: "ArrowDown" });
    await waitFor(() => {
      expect(within(list).getByRole("option", { name: /Gruppo di 10 volti/i })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });

    fireEvent.keyDown(window, { key: "n" });
    const who = await screen.findByRole("combobox", { name: "Chi è?" });
    expect(who).toHaveFocus();

    fireEvent.change(who, { target: { value: "Marco" } });
    await screen.findByRole("option", { name: /Crea l'identità "Marco"/i });
    fireEvent.keyDown(who, { key: "Enter" });

    await waitFor(() => expect(commandsMock.linkPersonIdentity).toHaveBeenCalledWith(1, "uid-new"));
    await waitFor(() => {
      expect(within(list).getByRole("option", { name: /Gruppo di 20 volti/i })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });
  });

  it("non attiva le scorciatoie mentre si scrive in un campo", async () => {
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 1, faces: 11 }), row({ id: 2, faces: 22 })],
      next: null,
      total: 2,
    } as never);
    renderModule();
    const list = await screen.findByTestId("group-list-scroll");
    await within(list).findByText(/Gruppo di 11 volti/i);
    fireEvent.keyDown(window, { key: "ArrowDown" });
    await waitFor(() =>
      expect(list.querySelector('[data-group-id="1"]')).toHaveAttribute("aria-selected", "true"),
    );

    const filter = screen.getByLabelText(/Filtra per nome/i);
    filter.focus();
    fireEvent.keyDown(filter, { key: "ArrowDown" });
    // Resta sul primo: ↓ nel campo filtro non deve spostare la selezione.
    expect(list.querySelector('[data-group-id="1"]')).toHaveAttribute("aria-selected", "true");
    expect(list.querySelector('[data-group-id="2"]')).toHaveAttribute("aria-selected", "false");
  });
});

describe("Aggiornamento dopo l'analisi", () => {
  it("su faces://changed aggiorna elenco e conteggi senza perdere selezione né scorrimento", async () => {
    commandsMock.listGroups.mockResolvedValue({
      items: [row({ id: 1, faces: 10, photos: 8 }), row({ id: 2, faces: 20, photos: 15 })],
      next: null,
      total: 2,
    } as never);
    const client = new QueryClient();
    const invalidate = vi.spyOn(client, "invalidateQueries");
    render(
      <QueryClientProvider client={client}>
        <PeopleModule />
      </QueryClientProvider>,
    );

    const list = await screen.findByTestId("group-list-scroll");
    await within(list).findByText(/Gruppo di 10 volti/i);
    fireEvent.click(within(list).getByText(/Gruppo di 20 volti/i));
    await waitFor(() =>
      expect(within(list).getByRole("option", { name: /Gruppo di 20 volti/i })).toHaveAttribute(
        "aria-selected",
        "true",
      ),
    );

    Object.defineProperty(list, "scrollTop", { value: 120, writable: true, configurable: true });
    expect(list.scrollTop).toBe(120);

    commandsMock.peopleSections.mockResolvedValue({
      review: 40,
      people: 13,
      ignored: 5,
      singles: 400,
      discarded: 20,
    } as never);
    commandsMock.listGroups.mockResolvedValue({
      items: [
        row({ id: 1, faces: 12, photos: 9 }),
        row({ id: 2, faces: 22, photos: 16 }),
        row({ id: 3, faces: 5, photos: 4 }),
      ],
      next: null,
      total: 3,
    } as never);

    act(() => emit("facesChanged", undefined));

    await waitFor(() => expect(invalidate).toHaveBeenCalledWith({ queryKey: ["people-sections"] }));
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["people-groups"] });
    await waitFor(() =>
      expect(within(list).getByRole("option", { name: /Gruppo di 22 volti/i })).toHaveAttribute(
        "aria-selected",
        "true",
      ),
    );
    expect(list.scrollTop).toBe(120);
    const tabs = screen.getByRole("tablist");
    await waitFor(() =>
      expect(within(tabs).getByRole("tab", { name: /Da rivedere/i })).toHaveTextContent("40"),
    );
  });
});

describe("Volti scartati senza gruppo", () => {
  it("compaiono in Da rivedere e si aprono con l'elenco dei volti scartati", async () => {
    renderModule();
    const entry = await screen.findByRole("button", { name: /18 volti scartati/i });
    fireEvent.click(entry);
    await waitFor(() =>
      expect(commandsMock.groupFaces).toHaveBeenCalledWith(
        { kind: "discarded", group: null },
        expect.anything(),
        null,
        expect.anything(),
      ),
    );
  });

  it("Apri in Persone su un volto scartato senza gruppo apre l'elenco dei scartati", async () => {
    commandsMock.locateFace.mockResolvedValue(
      located({ faceId: 55, section: "review", groupId: null, discarded: true }),
    );
    renderModule();
    act(() => {
      useLayout.getState().openPeople({ face: 55 });
    });
    await waitFor(() =>
      expect(commandsMock.groupFaces).toHaveBeenCalledWith(
        { kind: "discarded", group: null },
        expect.anything(),
        null,
        expect.anything(),
      ),
    );
  });
});
