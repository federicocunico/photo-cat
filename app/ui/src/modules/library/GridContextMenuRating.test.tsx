import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { GridContextMenu } from "./GridContextMenu";

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

function renderMenu() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <GridContextMenu
        x={0}
        y={0}
        ids={ids}
        target={target}
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
  await user.click(screen.getByRole("menuitem", { name: "Nessuna" }));
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

it("Ripristina data del file", async () => {
  const user = userEvent.setup();
  renderMenu();
  await user.click(screen.getByRole("menuitem", { name: "Ripristina data del file" }));
  expect(onBulk).toHaveBeenCalledWith(target, { action: "clear_manual_date" }, 3);
  expect(onClose).toHaveBeenCalled();
});
