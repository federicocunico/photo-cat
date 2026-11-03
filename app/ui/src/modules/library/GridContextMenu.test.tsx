import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { GridContextMenu } from "./GridContextMenu";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());

const cat = (id: number, name: string, kind: string, system: string | null = null) => ({
  catalog: { id, name, kind, rules: null, createdAt: 0, system },
  items: 0,
  excluded: 0,
});
const ids = Array.from({ length: 20 }, (_, i) => i + 1);
const onAdd = vi.fn();
const onClose = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listCatalogs.mockResolvedValue([
    cat(3, "Vacanze", "manual"),
    cat(2, "Amici", "manual"),
    cat(4, "Recenti", "smart"),
    cat(90, "Cestino", "manual", "trash"),
  ] as never);
});

function renderMenu() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <GridContextMenu
        x={0}
        y={0}
        ids={ids}
        target={{ kind: "ids", ids }}
        onOpen={vi.fn()}
        onPlace={vi.fn()}
        onAddToCatalog={onAdd}
        onClose={onClose}
      />
    </QueryClientProvider>,
  );
}
const openSub = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole("menuitem", { name: /Aggiungi a catalogo/ }));
  return screen.findByRole("combobox", { name: "Cerca un catalogo" });
};

it("aggiunge le 20 foto del menu al catalogo scelto; solo manuali, in ordine", async () => {
  const user = userEvent.setup();
  renderMenu();
  await openSub(user);
  await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(3));
  expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
    "Amici",
    "Vacanze",
    "Nuovo catalogo…",
  ]);
  expect(screen.queryByRole("option", { name: /Recenti|Cestino/ })).toBeNull();
  await user.click(screen.getByRole("option", { name: "Vacanze" }));
  expect(onAdd).toHaveBeenCalledWith({ kind: "ids", ids }, 20, 3, "Vacanze");
  expect(onClose).toHaveBeenCalled();
});

it("la ricerca filtra l'elenco", async () => {
  const user = userEvent.setup();
  renderMenu();
  const input = await openSub(user);
  await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(3));
  await user.type(input, "vac");
  expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["Vacanze"]);
});

it("Nuovo catalogo… crea il catalogo e vi aggiunge le foto", async () => {
  commandsMock.saveCatalog.mockResolvedValue(77 as never);
  const user = userEvent.setup();
  renderMenu();
  await openSub(user);
  await user.click(await screen.findByRole("option", { name: "Nuovo catalogo…" }));
  await user.type(
    await screen.findByRole("textbox", { name: "Nome del catalogo" }),
    "Estate{Enter}",
  );
  await waitFor(() =>
    expect(commandsMock.saveCatalog).toHaveBeenCalledWith(null, "Estate", "manual", null),
  );
  await waitFor(() => expect(onAdd).toHaveBeenCalledWith({ kind: "ids", ids }, 20, 77, "Estate"));
});

it("da tastiera: freccia destra apre il sottomenu e il focus va alla ricerca", async () => {
  const user = userEvent.setup();
  renderMenu();
  const trigger = screen.getByRole("menuitem", { name: /Aggiungi a catalogo/ });
  await waitFor(() => expect(screen.getAllByRole("menuitem")[0]).toHaveFocus());
  trigger.focus();
  await user.keyboard("{ArrowRight}");
  const input = await screen.findByRole("combobox", { name: "Cerca un catalogo" });
  await waitFor(() => expect(input).toHaveFocus());
  await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(3));
  await user.keyboard("{ArrowDown}{Enter}");
  expect(onAdd).toHaveBeenCalledWith({ kind: "ids", ids }, 20, 3, "Vacanze");
});
