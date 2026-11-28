import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { GridContextMenu } from "./GridContextMenu";
import { emptyFilter } from "../../store/library";
import type { Target } from "../../lib/ipc";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());

const ids = [1, 2, 3];
const target = { kind: "ids", ids } as const;
const onBulk = vi.fn();
const onClose = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listCatalogs.mockResolvedValue([] as never);
});

function renderMenu(
  menuIds: number[] = ids,
  qc = new QueryClient(),
  menuTarget: Target = { kind: "ids", ids: menuIds },
) {
  return render(
    <QueryClientProvider client={qc}>
      <GridContextMenu
        x={0}
        y={0}
        ids={menuIds}
        target={menuTarget}
        onOpen={vi.fn()}
        onPlace={vi.fn()}
        onAddToCatalog={vi.fn()}
        onBulk={onBulk}
        onClose={onClose}
      />
    </QueryClientProvider>,
  );
}

it("Valutazione: 4 stelle e Nessuna", async () => {
  const user = userEvent.setup();
  renderMenu();
  await user.click(screen.getByRole("menuitem", { name: /^Valutazione/ }));
  await user.click(screen.getByRole("menuitem", { name: "4 stelle" }));
  expect(onBulk).toHaveBeenCalledWith(target, { action: "set_rating", rating: 4 }, 3);
  await user.click(screen.getByRole("menuitem", { name: "Nessuna valutazione" }));
  expect(onBulk).toHaveBeenLastCalledWith(target, { action: "set_rating", rating: null }, 3);
});

it("Colore: Seppia e Non indicato", async () => {
  const user = userEvent.setup();
  renderMenu();
  await user.click(screen.getByRole("menuitem", { name: /^Colore/ }));
  await user.click(screen.getByRole("menuitem", { name: "Seppia" }));
  expect(onBulk).toHaveBeenCalledWith(
    target,
    { action: "set_color_trait", color_trait: "sepia" },
    3,
  );
  await user.click(screen.getByRole("menuitem", { name: "Non indicato" }));
  expect(onBulk).toHaveBeenLastCalledWith(
    target,
    { action: "set_color_trait", color_trait: null },
    3,
  );
});

it("Imposta data di scatto: dialogo con precisione mese", async () => {
  const user = userEvent.setup();
  renderMenu();
  await user.click(screen.getByRole("menuitem", { name: /Imposta data di scatto/ }));
  await user.click(await screen.findByRole("radio", { name: "Mese" }));
  const year = screen.getByLabelText("Anno");
  await user.clear(year);
  await user.type(year, "1999");
  const month = screen.getByLabelText("Mese", { selector: "input" });
  await user.clear(month);
  await user.type(month, "7");
  await user.click(screen.getByRole("button", { name: "Imposta" }));
  expect(onBulk).toHaveBeenCalledWith(
    target,
    { action: "set_manual_date", value: "1999-07", precision: "month" },
    3,
  );
});

const openDateDialog = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole("menuitem", { name: /Imposta data di scatto/ }));
  return screen.findByRole("button", { name: "Imposta" });
};

it("Data completa dal menu contestuale: 60 foto, parte su giorno con i campi vuoti", async () => {
  const user = userEvent.setup();
  const many = Array.from({ length: 60 }, (_, i) => i + 1);
  renderMenu(many);
  const set = await openDateDialog(user);
  expect(screen.getByRole("radio", { name: "Giorno" })).toBeChecked();
  expect(screen.getByLabelText("Anno")).toHaveValue(null);
  expect(screen.getByLabelText("Mese", { selector: "input" })).toHaveValue(null);
  expect(screen.getByLabelText("Giorno", { selector: "input" })).toHaveValue(null);
  // Campi vuoti: nessun errore, "Imposta" disattivato.
  expect(set).toBeDisabled();
  expect(screen.queryByText("Data non valida")).toBeNull();
  await user.type(screen.getByLabelText("Giorno", { selector: "input" }), "3");
  await user.type(screen.getByLabelText("Mese", { selector: "input" }), "3");
  await user.type(screen.getByLabelText("Anno"), "1987");
  await user.click(set);
  expect(onBulk).toHaveBeenCalledWith(
    { kind: "ids", ids: many },
    { action: "set_manual_date", value: "1987-03-03", precision: "day" },
    60,
  );
});

it("Una sola foto: il dialogo parte dalla data di oggi, come prima", async () => {
  const user = userEvent.setup();
  renderMenu([1]);
  const set = await openDateDialog(user);
  const now = new Date();
  expect(screen.getByRole("radio", { name: "Giorno" })).toBeChecked();
  expect(screen.getByLabelText("Anno")).toHaveValue(now.getFullYear());
  expect(screen.getByLabelText("Giorno", { selector: "input" })).toHaveValue(now.getDate());
  expect(set).toBeEnabled();
});

it("Data manuale comune già nota: il dialogo parte da giugno 1984", async () => {
  const user = userEvent.setup();
  const qc = new QueryClient();
  for (const id of ids)
    qc.setQueryData(["media-detail", id], {
      dates: {
        capturedAt: null,
        capturedSource: "manual",
        modifiedAt: null,
        manualAt: "1984-06-01 00:00:00",
        manualPrecision: "month",
      },
    });
  renderMenu(ids, qc);
  await openDateDialog(user);
  expect(screen.getByRole("radio", { name: "Mese" })).toBeChecked();
  expect(screen.getByLabelText("Anno")).toHaveValue(1984);
  expect(screen.getByLabelText("Mese", { selector: "input" })).toHaveValue(6);
});

it("Selezione per filtro: il dialogo parte vuoto", async () => {
  const user = userEvent.setup();
  renderMenu([1, 2], new QueryClient(), {
    kind: "filter",
    filter: emptyFilter(),
    maxId: 500,
    except: [],
  });
  await openDateDialog(user);
  expect(screen.getByLabelText("Anno")).toHaveValue(null);
});

it("Ripristina data del file", async () => {
  const user = userEvent.setup();
  renderMenu();
  await user.click(screen.getByRole("menuitem", { name: "Ripristina data del file" }));
  expect(onBulk).toHaveBeenCalledWith(target, { action: "clear_manual_date" }, 3);
  expect(onClose).toHaveBeenCalled();
});
