import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it } from "vitest";
import "../../i18n";
import { askConfirm } from "../../store/confirm";
import { ConfirmHost } from "./ConfirmHost";
import { checkedCleanup } from "../../test/a11y";

afterEach(checkedCleanup());

it("askConfirm apre il dialogo condiviso e risolve con la scelta", async () => {
  const user = userEvent.setup();
  render(<ConfirmHost />);
  let answer: Promise<boolean>;
  act(() => {
    answer = askConfirm({ title: "Aprire 40 file insieme?", confirmLabel: "Apri 40 file" });
  });
  const dialog = await screen.findByRole("alertdialog");
  await user.click(within(dialog).getByRole("button", { name: "Apri 40 file" }));
  await expect(answer!).resolves.toBe(true);
  await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());

  act(() => {
    answer = askConfirm({ title: "Aprire 40 file insieme?", confirmLabel: "Apri 40 file" });
  });
  await user.keyboard("{Escape}");
  await expect(answer!).resolves.toBe(false);
});
