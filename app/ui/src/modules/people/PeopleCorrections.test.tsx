import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import type { FaceView, GroupRow } from "../../lib/ipc";
import { ToastHost } from "../../components/feedback/ToastHost";
import { PeopleModule } from "./PeopleModule";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

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
    personId: 12,
    ...partial,
  };
}

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

function renderModule() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <PeopleModule />
      <ToastHost />
    </QueryClientProvider>,
  );
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
    review: 2,
    people: 1,
    ignored: 1,
    singles: 0,
    discarded: 0,
  } as never);
  commandsMock.groupFaces.mockResolvedValue({
    items: [face({ id: 1 }), face({ id: 2 })],
    next: null,
    total: 2,
    goodCount: 2,
    discardedCount: 0,
  } as never);
  commandsMock.suggestIdentities.mockResolvedValue([] as never);
  commandsMock.listIdentities.mockResolvedValue({
    identities: [],
    groups: [],
    storePath: "C:\\app.db",
    defaultThreshold: 0.363,
  } as never);
  commandsMock.correctPerson.mockResolvedValue(null);
  commandsMock.undoPeople.mockResolvedValue(null);
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

describe("Unione persone", () => {
  it("apre il dialogo e unisce «Senza nome» con Giulia", async () => {
    const unnamed = row({ id: 12, faces: 61, photos: 58, section: "review" });
    const giulia = row({
      id: 7,
      name: "Giulia",
      identityUid: "uid-g",
      faces: 412,
      photos: 412,
      section: "people",
    });
    commandsMock.listGroups.mockImplementation(async (section: string) => {
      if (section === "review") return { items: [unnamed], next: null, total: 1 };
      if (section === "people") return { items: [giulia], next: null, total: 1 };
      return { items: [], next: null, total: 0 };
    });

    renderModule();
    fireEvent.click(await screen.findByText(/Gruppo di 61 volti/i));

    const mergeSelect = await screen.findByLabelText(/Gruppo o persona di destinazione/i);
    await waitFor(() =>
      expect(
        [...(mergeSelect as HTMLSelectElement).options].some((o) =>
          o.textContent?.includes("Giulia"),
        ),
      ).toBe(true),
    );
    fireEvent.change(mergeSelect, { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: /^Unisci con$/i }));

    const dialog = await screen.findByRole("dialog", { name: /Unisci gruppi/i });
    expect(within(dialog).getByText(/61 volti diventano di «Giulia»/i)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: /Unisci a Giulia/i }));

    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "merge",
        from: 12,
        to: 7,
      }),
    );
  });

  it("chiede quale nome tenere quando entrambi sono nominati", async () => {
    const marco = row({
      id: 3,
      name: "Marco",
      identityUid: "uid-m",
      faces: 20,
      photos: 18,
      section: "review",
    });
    const giulia = row({
      id: 7,
      name: "Giulia",
      identityUid: "uid-g",
      faces: 412,
      photos: 412,
      section: "people",
    });
    // Un gruppo "review" con nome non è tipico, ma copre la scelta del nome nel dialogo.
    commandsMock.listGroups.mockImplementation(async (section: string) => {
      if (section === "review") return { items: [marco], next: null, total: 1 };
      if (section === "people") return { items: [giulia], next: null, total: 1 };
      return { items: [], next: null, total: 0 };
    });

    renderModule();
    fireEvent.click(await screen.findByText("Marco"));
    const mergeSelect = await screen.findByLabelText(/Gruppo o persona di destinazione/i);
    await waitFor(() =>
      expect(
        [...(mergeSelect as HTMLSelectElement).options].some((o) =>
          o.textContent?.includes("Giulia"),
        ),
      ).toBe(true),
    );
    fireEvent.change(mergeSelect, { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: /^Unisci con$/i }));

    const dialog = await screen.findByRole("dialog", { name: /Unisci gruppi/i });
    expect(within(dialog).getByText(/Quale nome tenere/i)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByLabelText("Marco"));
    fireEvent.click(within(dialog).getByRole("button", { name: /Unisci a Marco/i }));

    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "merge",
        from: 7,
        to: 3,
      }),
    );
  });
});

describe("Persone diverse", () => {
  it("segna la coppia e permette di annullare", async () => {
    const g = row({
      id: 1,
      faces: 40,
      photos: 35,
      mergeHint: {
        otherId: 9,
        otherName: null,
        otherFaces: 17,
        otherPhotos: 15,
        otherCover: null,
        score: 0.58,
      },
    });
    commandsMock.listGroups.mockResolvedValue({
      items: [g],
      next: null,
      total: 1,
    } as never);

    renderModule();
    fireEvent.click(await screen.findByText(/Gruppo di 40 volti/i));
    expect(await screen.findByText(/Somiglia a/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Sono persone diverse/i }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "not_same",
        a: 1,
        b: 9,
      }),
    );

    const toast = await screen.findByRole("status");
    expect(toast).toHaveTextContent(/persone diverse/i);
    fireEvent.click(within(toast).getByRole("button", { name: /^Annulla$/i }));
    await waitFor(() => expect(commandsMock.undoPeople).toHaveBeenCalledTimes(1));
  });
});

describe("Ripristino", () => {
  it("mostra +N volti nuovi e ripristina dalla sezione Ignorati", async () => {
    const ignored = row({
      id: 40,
      faces: 146,
      photos: 140,
      section: "ignored",
      ignoredAt: Date.UTC(2026, 10, 2),
      newFaces: 6,
    });
    commandsMock.listGroups.mockImplementation(async (section: string) => {
      if (section === "ignored") return { items: [ignored], next: null, total: 1 };
      return { items: [], next: null, total: 0 };
    });

    renderModule();
    const tabs = await screen.findByRole("tablist");
    fireEvent.click(within(tabs).getByRole("tab", { name: /Ignorati/i }));

    expect(await screen.findByText(/Gruppo di 146 volti/i)).toBeInTheDocument();
    expect(screen.getByText(/\+6 volti nuovi/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^Ripristina$/i }));
    await waitFor(() =>
      expect(commandsMock.correctPerson).toHaveBeenCalledWith({
        action: "restore",
        group: 40,
      }),
    );
  });
});
