import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { IdentitiesView, IdentityRow } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { IdentitiesPage } from "./IdentitiesPage";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => "D:\\ref.jpg"),
  save: vi.fn(),
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

function renderPage() {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <IdentitiesPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
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
afterEach(cleanup);

it("lists identities with group filter, link state and store location", async () => {
  renderPage();
  const table = await screen.findByRole("table", { name: "Identità" });
  expect(await within(table).findByText("Giulia Bianchi")).toBeInTheDocument();
  expect(within(table).getByText("412")).toBeInTheDocument();
  expect(within(table).getByText("non collegata")).toBeInTheDocument();
  expect(within(table).getByText("nessuno")).toBeInTheDocument();
  expect(screen.getByRole("note")).toHaveTextContent("app.db");
  fireEvent.click(screen.getByRole("button", { name: "Lavoro (1)" }));
  expect(within(table).queryByText("Giulia Bianchi")).toBeNull();
  expect(within(table).getByText("Elena Conti")).toBeInTheDocument();
});

it("finds an identity by alias", async () => {
  renderPage();
  await screen.findByText("Giulia Bianchi");
  fireEvent.change(screen.getByLabelText("Cerca nome o alias"), {
    target: { value: "Nonna Rita" },
  });
  await waitFor(() => expect(commandsMock.listIdentities).toHaveBeenCalledWith("Nonna Rita"));
  const table = screen.getByRole("table", { name: "Identità" });
  await waitFor(() => expect(within(table).queryByText("Giulia Bianchi")).toBeNull());
  expect(within(table).getByText("Rita Rossi")).toBeInTheDocument();
});

it("creates an identity, leaving the short name to the default", async () => {
  commandsMock.saveIdentity.mockResolvedValueOnce("new-uid");
  renderPage();
  fireEvent.click(await screen.findByText("+ Nuova identità"));
  fireEvent.change(screen.getByLabelText("Nome visualizzato"), {
    target: { value: "Marco Russo" },
  });
  expect(screen.getByLabelText("Nome breve (per i file)")).toHaveAttribute("placeholder", "Marco");
  fireEvent.change(screen.getByLabelText("Alias"), { target: { value: "Marco R., Il Russo" } });
  fireEvent.click(screen.getByRole("button", { name: "Crea identità" }));
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

it("shows the conflict when the name already exists", async () => {
  commandsMock.saveIdentity.mockRejectedValueOnce({
    code: "INVALID_INPUT",
    detail: 'conflitto: esiste già un\'identità "Giulia Bianchi"',
  });
  renderPage();
  fireEvent.click(await screen.findByText("+ Nuova identità"));
  fireEvent.change(screen.getByLabelText("Nome visualizzato"), {
    target: { value: "Giulia Bianchi" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Crea identità" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("esiste già");
});

it("asks which face to use when the reference image has several", async () => {
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
  renderPage();
  fireEvent.click(await screen.findByText("Giulia Bianchi"));
  fireEvent.click(screen.getByRole("button", { name: "+ Da file…" }));
  const group = await screen.findByRole("group", { name: /più volti/ });
  expect(within(group).getAllByRole("button")).toHaveLength(3);
  fireEvent.click(within(group).getByRole("button", { name: "Volto 2" }));
  await waitFor(() =>
    expect(commandsMock.addIdentityReferenceFile).toHaveBeenLastCalledWith("g", "D:\\ref.jpg", 1),
  );
});
