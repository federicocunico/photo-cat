import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { IdentitiesView, IdentityRow } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { PeopleModule } from "./PeopleModule";
import { deferred } from "../../test/deferred";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => "D:\\ref.jpg"),
  save: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const row = (p: Partial<IdentityRow> & { uid: string; name: string }): IdentityRow => ({
  shortName: p.name.split(" ")[0],
  group: null,
  threshold: null,
  aliases: [],
  faces: 0,
  createdAt: Date.UTC(2026, 2, 12),
  person: null,
  photos: 0,
  ...p,
});

const view = (): IdentitiesView => ({
  identities: [
    row({
      uid: "g",
      name: "Giulia Bianchi",
      group: "Famiglia",
      aliases: ["Giuli"],
      faces: 8,
      person: 4,
      photos: 412,
    }),
    row({
      uid: "r",
      name: "Rita Rossi",
      group: "Famiglia",
      aliases: ["Nonna Rita"],
      faces: 5,
      person: 5,
      photos: 140,
    }),
    row({ uid: "e", name: "Elena Conti", group: "Lavoro" }),
  ],
  groups: ["Famiglia", "Lavoro"],
  storePath: "C:\\Users\\me\\AppData\\Roaming\\photocat\\photo-cat\\config\\app.db",
  defaultThreshold: 0.363,
});

function renderModule() {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <PeopleModule />
    </QueryClientProvider>,
  );
}

async function openPeopleSection() {
  renderModule();
  const tabs = await screen.findByRole("tablist");
  fireEvent.click(within(tabs).getByRole("tab", { name: /^Persone/i }));
  await screen.findByText("Giulia Bianchi");
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
    review: 0,
    people: 3,
    ignored: 0,
    singles: 0,
    discarded: 0,
  } as never);
  commandsMock.listGroups.mockResolvedValue({ items: [], next: null, total: 0 } as never);
  commandsMock.listIdentities.mockImplementation(async (q: string | null = null) => {
    const v = view();
    if (!q) return v;
    const l = q.toLowerCase();
    return {
      ...v,
      identities: v.identities.filter(
        (i) =>
          i.name.toLowerCase().includes(l) || i.aliases.some((a) => a.toLowerCase().includes(l)),
      ),
    };
  });
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

const people = () => screen.getByRole("listbox", { name: "Persone" });

it("elenca le persone con il filtro per cerchia, le foto nella libreria e l'archivio", async () => {
  await openPeopleSection();
  const table = people();
  expect(within(table).getByText("Giulia Bianchi")).toBeInTheDocument();
  expect(within(table).getByText(/412 foto$/)).toBeInTheDocument();
  expect(within(table).getByText(/nessuna foto in questa libreria$/)).toBeInTheDocument();
  expect(table).not.toHaveTextContent(/nessuno|non collegata/);
  expect(screen.getByRole("note")).toHaveTextContent("app.db");
  // La cerchia è nel menu "Ordina e filtra", con i conteggi.
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Ordina e filtra" }));
  await user.click(await screen.findByRole("menuitemcheckbox", { name: "Lavoro (1)" }));
  await waitFor(() => expect(within(table).queryByText("Giulia Bianchi")).toBeNull());
  expect(within(table).getByText("Elena Conti")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Ordina e filtra" })).toHaveAttribute("data-active");
});

it("trova una persona da un altro nome", async () => {
  await openPeopleSection();
  fireEvent.change(screen.getByLabelText("Cerca un nome o un altro nome"), {
    target: { value: "Nonna Rita" },
  });
  await waitFor(() => expect(commandsMock.listIdentities).toHaveBeenCalledWith("Nonna Rita", null));
  await waitFor(() => expect(within(people()).queryByText("Giulia Bianchi")).toBeNull());
  expect(within(people()).getByText("Rita Rossi")).toBeInTheDocument();
});

it("«Nuova persona» è nell'intestazione e crea la persona lasciando il nome nei file predefinito", async () => {
  commandsMock.saveIdentity.mockResolvedValueOnce("new-uid");
  await openPeopleSection();
  fireEvent.click(screen.getByRole("button", { name: "Nuova persona" }));
  fireEvent.change(screen.getByLabelText("Nome"), { target: { value: "Marco Russo" } });
  expect(screen.getByLabelText("Nome nei file esportati")).toHaveAttribute("placeholder", "Marco");
  fireEvent.change(screen.getByLabelText("Altri nomi"), {
    target: { value: "Marco R., Il Russo" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Crea la persona" }));
  await waitFor(() =>
    expect(commandsMock.saveIdentity).toHaveBeenCalledWith(null, {
      name: "Marco Russo",
      shortName: null,
      group: null,
      threshold: null,
      aliases: ["Marco R.", "Il Russo"],
    }),
  );
});

it("«Nuova persona» compare solo nella sezione Persone", async () => {
  renderModule();
  await screen.findByRole("tablist");
  expect(screen.queryByRole("button", { name: "Nuova persona" })).toBeNull();
});

it("mostra il conflitto quando il nome esiste già", async () => {
  commandsMock.saveIdentity.mockRejectedValueOnce({
    code: "INVALID_INPUT",
    detail: "conflitto: esiste già una persona «Giulia Bianchi»",
  });
  await openPeopleSection();
  fireEvent.click(screen.getByRole("button", { name: "Nuova persona" }));
  fireEvent.change(screen.getByLabelText("Nome"), { target: { value: "Giulia Bianchi" } });
  fireEvent.click(screen.getByRole("button", { name: "Crea la persona" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("esiste già");
});

it("i dettagli si salvano appena modificati, con «Salvato» accanto", async () => {
  commandsMock.saveIdentity.mockResolvedValue("g");
  await openPeopleSection();
  fireEvent.click(screen.getByText("Giulia Bianchi"));
  const aliases = await screen.findByLabelText("Altri nomi");
  expect(aliases).toHaveValue("Giuli");
  expect(screen.queryByRole("button", { name: "Salva" })).toBeNull();
  fireEvent.change(aliases, { target: { value: "Giuli, Giugi" } });
  fireEvent.blur(aliases);
  await waitFor(() =>
    expect(commandsMock.saveIdentity).toHaveBeenCalledWith("g", {
      name: "Giulia Bianchi",
      shortName: "Giulia",
      group: "Famiglia",
      threshold: null,
      aliases: ["Giuli", "Giugi"],
    }),
  );
  expect(await screen.findByText("Salvato")).toBeInTheDocument();
  // Uscire da un campo non modificato non salva di nuovo.
  fireEvent.blur(screen.getByLabelText("Nome"));
  expect(commandsMock.saveIdentity).toHaveBeenCalledOnce();
});

it("la sensibilità del riconoscimento è una scala senza numeri", async () => {
  await openPeopleSection();
  fireEvent.click(screen.getByText("Giulia Bianchi"));
  const slider = await screen.findByRole("slider", { name: "Riconoscimento" });
  expect(slider).toHaveAttribute("aria-valuetext", "Predefinito");
  expect(screen.getByText("Più risultati")).toBeInTheDocument();
  expect(screen.getByText("Più precisi")).toBeInTheDocument();
  expect(screen.queryByText("0,36")).toBeNull();
});

it("i volti di riferimento sono una scheda accanto ai volti nella libreria", async () => {
  commandsMock.addIdentityReferenceFile
    .mockResolvedValueOnce({
      outcome: "choose",
      faces: [0, 1, 2].map((index) => ({
        index,
        crop: "data:image/png;base64,",
        bbox: [0, 0, 0.1, 0.1],
      })),
    })
    .mockResolvedValueOnce({ outcome: "added", id: 9 });
  await openPeopleSection();
  fireEvent.click(screen.getByText("Giulia Bianchi"));
  const faces = await screen.findByRole("tablist", { name: "Volti" });
  expect(
    within(faces).getByRole("tab", { name: "Volti nella libreria · 412" }),
  ).toBeInTheDocument();
  const refs = within(faces).getByRole("tab", { name: "Volti di riferimento · 8" });
  fireEvent.mouseDown(refs, { button: 0 });
  fireEvent.click(await screen.findByRole("button", { name: "Da un file…" }));
  const group = await screen.findByRole("group", { name: /più volti/ });
  expect(within(group).getAllByRole("button")).toHaveLength(3);
  fireEvent.click(within(group).getByRole("button", { name: "Volto 2" }));
  await waitFor(() =>
    expect(commandsMock.addIdentityReferenceFile).toHaveBeenLastCalledWith("g", "D:\\ref.jpg", 1),
  );
});

it("una persona senza foto nella libreria resta modificabile; ⋯ offre Unisci, Esporta ed Elimina", async () => {
  await openPeopleSection();
  fireEvent.click(within(people()).getByText("Elena Conti"));
  expect(await screen.findByLabelText("Nome")).toHaveValue("Elena Conti");
  const card = screen.getByRole("complementary", { name: "Scheda" });
  expect(within(card).getByText(/nessuna foto in questa libreria/)).toBeInTheDocument();
  expect(within(card).queryByRole("button", { name: "Mostra in Libreria" })).toBeNull();
  const user = userEvent.setup();
  await user.click(within(card).getByRole("button", { name: "Altre azioni per Elena Conti" }));
  expect(await screen.findByRole("menuitem", { name: "Unisci a…" })).toBeInTheDocument();
  expect(screen.getByRole("menuitem", { name: "Esporta…" })).toBeInTheDocument();
  expect(screen.getByRole("menuitem", { name: "Elimina persona…" })).toBeInTheDocument();
});

it("«Esporta…» dal menu della persona esporta solo lei", async () => {
  const dialog = await import("@tauri-apps/plugin-dialog");
  vi.mocked(dialog.save).mockResolvedValueOnce("D:\\out\\Giulia.photocat-people.json");
  await openPeopleSection();
  fireEvent.click(screen.getByText("Giulia Bianchi"));
  const card = screen.getByRole("complementary", { name: "Scheda" });
  const user = userEvent.setup();
  await user.click(
    await within(card).findByRole("button", { name: "Altre azioni per Giulia Bianchi" }),
  );
  await user.click(await screen.findByRole("menuitem", { name: "Esporta…" }));
  await waitFor(() =>
    expect(commandsMock.exportIdentities).toHaveBeenCalledWith(
      "D:\\out\\Giulia.photocat-people.json",
      true,
      ["g"],
    ),
  );
});

it("Doppio clic su «Crea la persona»: il salvataggio parte una sola volta", async () => {
  const pending = deferred<string>();
  commandsMock.saveIdentity.mockImplementationOnce(() => pending.promise);
  await openPeopleSection();
  fireEvent.click(screen.getByRole("button", { name: "Nuova persona" }));
  fireEvent.change(screen.getByLabelText("Nome"), { target: { value: "Test persona" } });
  const save = screen.getByRole("button", { name: "Crea la persona" });
  fireEvent.click(save);
  fireEvent.click(save);
  fireEvent.submit(save.closest("form")!);
  await waitFor(() => expect(save).toHaveAttribute("aria-busy", "true"));
  expect(commandsMock.saveIdentity).toHaveBeenCalledOnce();
  await act(async () => pending.resolve("new-uid"));
  await waitFor(() => expect(commandsMock.listIdentities.mock.calls.length).toBeGreaterThan(1));
  expect(commandsMock.saveIdentity).toHaveBeenCalledOnce();
});
