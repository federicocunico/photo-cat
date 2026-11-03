import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { useMoveToTrash } from "./useMoveToTrash";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());
beforeEach(() => vi.clearAllMocks());

function Harness() {
  const trash = useMoveToTrash();
  return (
    <>
      <button onClick={() => void trash.moveToTrash({ kind: "ids", ids: [1, 2, 3] }, 3)}>go</button>
      {trash.dialog}
    </>
  );
}
const renderIt = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Harness />
    </QueryClientProvider>,
  );

it("senza cataloghi coinvolti parte subito, senza dialogo", async () => {
  const user = userEvent.setup();
  renderIt();
  await user.click(screen.getByText("go"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [1, 2, 3] },
      { action: "move_to_trash" },
      false,
    ),
  );
  expect(screen.queryByRole("alertdialog")).toBeNull();
});

it("con cataloghi mostra l'elenco; Annulla non avvia nulla", async () => {
  commandsMock.trashImpact.mockResolvedValue([
    { name: "Mare", items: 3 },
    { name: "Monti", items: 1 },
  ]);
  const user = userEvent.setup();
  renderIt();
  await user.click(screen.getByText("go"));
  const dialog = await screen.findByRole("alertdialog");
  expect(dialog).toHaveTextContent("Mare — 3 foto");
  expect(dialog).toHaveTextContent("Monti — 1 foto");
  await user.click(screen.getByRole("button", { name: "Annulla" }));
  await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
});

it("il focus iniziale è sulla conferma: Invio sposta nel cestino", async () => {
  commandsMock.trashImpact.mockResolvedValue([{ name: "Mare", items: 3 }]);
  const user = userEvent.setup();
  renderIt();
  await user.click(screen.getByText("go"));
  const confirm = await screen.findByRole("button", { name: "Sposta nel cestino" });
  await waitFor(() => expect(confirm).toHaveFocus());
  await user.keyboard("{Enter}");
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      { kind: "ids", ids: [1, 2, 3] },
      { action: "move_to_trash" },
      false,
    ),
  );
});
