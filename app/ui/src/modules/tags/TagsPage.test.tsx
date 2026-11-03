import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { TagOverview } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { selIds, useLibrary } from "../../store/library";
import { useGuide } from "../../store/guide";
import { useToast } from "../../store/toasts";
import { runUndo, useUndo } from "../../store/undo";
import { TagsPage } from "./TagsPage";
import { deferred } from "../../test/deferred";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const overview = (): TagOverview => ({
  tags: [
    { name: "Città", count: 201, color: "teal", sets: ["Viaggio USA 2026"] },
    { name: "Paesaggi", count: 312, color: "blue", sets: ["Viaggio USA 2026"] },
    { name: "Scartate", count: 410, color: "gray", sets: [] },
    { name: "Sunset", count: 12, color: "coral", sets: [] },
    { name: "Tramonti", count: 38, color: "yellow", sets: ["Base", "Viaggio USA 2026"] },
  ],
  sets: [
    {
      id: 1,
      name: "Base",
      exclusive: false,
      isBase: true,
      exportKeywords: false,
      tags: [{ key: 1, name: "Tramonti" }],
      tagged: 38,
    },
    {
      id: 2,
      name: "Viaggio USA 2026",
      exclusive: true,
      isBase: false,
      exportKeywords: false,
      tags: [
        { key: 1, name: "Paesaggi" },
        { key: 2, name: "Città" },
        { key: 3, name: "Tramonti" },
      ],
      tagged: 520,
    },
  ],
  activeSet: 2,
  palette: ["blue", "teal", "yellow", "coral", "purple", "gray"],
});

function renderPage() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <TagsPage />
    </QueryClientProvider>,
  );
}

const messages = () => useToast.getState().notices.map((n) => n.message);

/** Sceglie il tag nell'elenco di tutti i tag e apre la modifica del suo nome. */
async function editTagName(name: string) {
  fireEvent.click((await screen.findAllByRole("button", { name: new RegExp(`^${name}`) }))[0]);
  const panel = screen.getByRole("complementary", { name: "Tag selezionato" });
  fireEvent.click(within(panel).getByRole("button", { name: `Nome: ${name}. Modifica` }));
  return within(panel).getByRole("textbox", { name: "Nome" });
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.tagOverview.mockResolvedValue(overview());
  useUndo.setState({ last: null, busy: false });
  useToast.setState({ notices: [] });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

it("shows the active set with keys and counts", async () => {
  renderPage();
  expect(await screen.findByRole("heading", { name: /Viaggio USA 2026/ })).toBeInTheDocument();
  const table = screen.getByRole("table", { name: "Viaggio USA 2026" });
  expect(within(table).getByText("Città")).toBeInTheDocument();
  expect(within(table).getByText("201")).toBeInTheDocument();
  expect(screen.getByText("3 tag · 520 foto taggate")).toBeInTheDocument();
  // Pulsanti a icona con etichetta accessibile.
  expect(screen.getByRole("button", { name: "Togli Città dal set" })).toBeInTheDocument();
});

it("un solo modo di rinominare: il nome modificabile sul campo", async () => {
  renderPage();
  await screen.findByRole("table", { name: "Viaggio USA 2026" });
  expect(screen.queryAllByRole("button", { name: /^Rinomina/ })).toEqual([]);
});

it("Doppio clic: a pending tag rename submits only once", async () => {
  renderPage();
  const name = await editTagName("Paesaggi");
  fireEvent.change(name, { target: { value: "Panorami" } });
  const pending = deferred<Awaited<ReturnType<typeof commandsMock.renameTag>>>();
  commandsMock.renameTag.mockImplementationOnce(() => pending.promise);
  const save = screen.getByRole("button", { name: "Salva" });
  fireEvent.click(save);
  fireEvent.click(save);
  expect(commandsMock.renameTag).toHaveBeenCalledOnce();
  expect(save).toHaveAttribute("aria-busy", "true");
  await act(async () => pending.resolve({ outcome: "renamed" } as never));
});

it("Annullamento: la rinomina si annulla dallo store undo (avviso e Ctrl+Z)", async () => {
  commandsMock.renameTag.mockResolvedValueOnce({ outcome: "renamed" } as never);
  renderPage();
  const name = await editTagName("Paesaggi");
  // La nota non rimanda più a un "Annulla" della Libreria.
  expect(screen.getByText(/Annulla operazione.*Ctrl\+Z/)).toBeInTheDocument();
  fireEvent.change(name, { target: { value: "Panorami" } });
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  await waitFor(() => expect(commandsMock.renameTag).toHaveBeenCalledWith("Paesaggi", "Panorami"));
  await waitFor(() => expect(messages()).toContain("Tag «Paesaggi» rinominato in «Panorami»"));
  const notice = useToast.getState().notices.find((n) => n.message.includes("Panorami"))!;
  expect(notice.action?.label).toBe("Annulla operazione");
  expect(useUndo.getState().last?.label).toBe("Tag «Paesaggi» rinominato in «Panorami»");
  await act(async () => {
    await runUndo();
  });
  expect(commandsMock.undoBulk).toHaveBeenCalledOnce();
});

it("proposes a merge next to the name when the new name exists, and merges only on request", async () => {
  commandsMock.renameTag.mockResolvedValueOnce({ outcome: "needsMerge", target: "Tramonti" });
  renderPage();
  const name = await editTagName("Sunset");
  fireEvent.change(name, { target: { value: "tramonti" } });
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  const panel = screen.getByRole("complementary", { name: "Tag selezionato" });
  const alert = await within(panel).findByText(/Esiste già il tag «Tramonti»/);
  expect(screen.queryByRole("alertdialog")).toBeNull();
  expect(commandsMock.mergeTag).not.toHaveBeenCalled();
  const box = alert.closest(".ui-alert") as HTMLElement;
  fireEvent.click(within(box).getByRole("button", { name: "Unisci" }));
  await waitFor(() => expect(commandsMock.mergeTag).toHaveBeenCalledWith("Sunset", "Tramonti"));
  await waitFor(() => expect(messages()).toContain("Tag «Sunset» unito in «Tramonti»"));
});

it("merges into another tag without a dialog, with undo", async () => {
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: /^Sunset/ }));
  fireEvent.click(screen.getByText("Unisci in un altro tag…"));
  fireEvent.change(screen.getByLabelText("Tag di destinazione"), {
    target: { value: "Tramonti" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Unisci" }));
  await waitFor(() => expect(commandsMock.mergeTag).toHaveBeenCalledWith("Sunset", "Tramonti"));
  expect(screen.queryByRole("alertdialog")).toBeNull();
  await waitFor(() => expect(useUndo.getState().last).not.toBeNull());
});

it("deletes a tag at once and offers undo", async () => {
  const confirm = vi.spyOn(window, "confirm");
  renderPage();
  fireEvent.click(await screen.findByRole("button", { name: /^Scartate/ }));
  fireEvent.click(screen.getByRole("button", { name: "Elimina tag (da 410 foto)" }));
  await waitFor(() => expect(commandsMock.deleteTag).toHaveBeenCalledWith("Scartate", true));
  await waitFor(() => expect(messages()).toContain("Tag «Scartate» eliminato"));
  await act(async () => {
    await runUndo();
  });
  expect(commandsMock.undoBulk).toHaveBeenCalledOnce();
  expect(confirm).not.toHaveBeenCalled();
});

it("deletes a set only after ConfirmDialog", async () => {
  renderPage();
  await screen.findByRole("table", { name: "Viaggio USA 2026" });
  fireEvent.click(screen.getByRole("button", { name: "Elimina set" }));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent("Eliminare il set «Viaggio USA 2026»?");
  expect(commandsMock.deleteTagSet).not.toHaveBeenCalled();
  fireEvent.click(within(dialog).getByRole("button", { name: "Annulla" }));
  await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  expect(commandsMock.deleteTagSet).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Elimina set" }));
  fireEvent.click(
    within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Elimina set" }),
  );
  await waitFor(() => expect(commandsMock.deleteTagSet).toHaveBeenCalledWith(2));
  await waitFor(() => expect(messages()).toContain("Set «Viaggio USA 2026» eliminato"));
});

it("renames a set from its name", async () => {
  renderPage();
  fireEvent.click(
    await screen.findByRole("button", { name: "Nome del set: Viaggio USA 2026. Modifica" }),
  );
  fireEvent.change(screen.getByRole("textbox", { name: "Nome del set" }), {
    target: { value: "USA" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  await waitFor(() =>
    expect(commandsMock.saveTagSet).toHaveBeenCalledWith(2, "USA", [
      "Paesaggi",
      "Città",
      "Tramonti",
    ]),
  );
});

it("creates a new set", async () => {
  commandsMock.saveTagSet.mockResolvedValueOnce(7);
  renderPage();
  fireEvent.click(await screen.findByText("+ Nuovo set"));
  fireEvent.change(screen.getByLabelText("Nome del nuovo set"), { target: { value: "Eventi" } });
  fireEvent.click(screen.getByRole("button", { name: "Crea" }));
  await waitFor(() => expect(commandsMock.saveTagSet).toHaveBeenCalledWith(null, "Eventi", []));
});

it("moves between sets with the arrow keys", async () => {
  renderPage();
  const active = await screen.findByRole("button", { name: /Viaggio USA 2026/, pressed: true });
  active.focus();
  fireEvent.keyDown(active, { key: "ArrowUp" });
  const base = await screen.findByRole("button", { name: /^Base/, pressed: true });
  expect(base).toHaveFocus();
  expect(screen.getByRole("heading", { name: /Base/ })).toBeInTheDocument();
});

it("reports conflicts when a set becomes exclusive and opens them in Library", async () => {
  const o = overview();
  o.sets[1].exclusive = false;
  commandsMock.tagOverview.mockResolvedValue(o);
  commandsMock.setTagSetExclusive.mockResolvedValueOnce([4, 5, 6, 7, 8, 9]);
  renderPage();
  fireEvent.click(await screen.findByLabelText(/^Esclusivo/));
  expect(await screen.findByText(/6 file hanno già due o più tag/)).toBeInTheDocument();
  fireEvent.click(screen.getByText("Apri in Libreria"));
  expect(selIds(useLibrary.getState().selection)).toEqual([4, 5, 6, 7, 8, 9]);
  expect(useLayout.getState().module).toBe("library");
});

it("il titolo della pagina è quello della voce di navigazione", async () => {
  renderPage();
  expect(await screen.findByRole("heading", { level: 1, name: "Tag" })).toBeInTheDocument();
});

it("mantiene il suggerimento al primo uso accanto ai set", async () => {
  useGuide.getState().resetTips();
  renderPage();
  expect(await screen.findByText(/Un set assegna i tasti 1–9/)).toBeInTheDocument();
});
