import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { open } from "@tauri-apps/plugin-dialog";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { useLayout } from "../../store/layout";
import { usePalette } from "../../store/palette";
import { useCommandStore } from "../../commands/registry";
import { usePaletteCommand } from "../../commands/paletteCommand";
import { ModuleNav } from "../shell/ModuleNav";
import { CommandPalette } from "./CommandPalette";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
afterEach(checkedCleanup());
beforeEach(() => {
  vi.mocked(open).mockReset();
  useCommandStore.setState({ commands: [] });
  usePalette.setState({ open: false, initial: "", recent: [] });
  useLayout.setState({ module: "library", rawReviewTarget: null });
});

function Shell() {
  usePaletteCommand();
  return (
    <>
      <ModuleNav />
      <CommandPalette />
    </>
  );
}

async function runCommand() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Shell />
    </QueryClientProvider>,
  );
  const user = userEvent.setup();
  await user.keyboard("{Control>}k{/Control}");
  await user.type(await screen.findByRole("combobox"), "Varianti RAW: apri");
  await user.click(await screen.findByRole("option", { name: /^Varianti RAW: apri cartella/ }));
}

describe("comando «Varianti RAW: apri cartella…»", () => {
  it("apre il dialogo di scelta cartella e porta la cartella nel modulo", async () => {
    vi.mocked(open).mockResolvedValue("D:/Foto/Dolomiti");
    await runCommand();
    expect(vi.mocked(open).mock.calls[0][0]).toMatchObject({ directory: true });
    await vi.waitFor(() => expect(useLayout.getState().module).toBe("raw-review"));
    expect(useLayout.getState().rawReviewTarget).toEqual({
      sessionId: undefined,
      root: "D:/Foto/Dolomiti",
    });
  });

  it("annullando il dialogo apre comunque il modulo, senza cartella", async () => {
    vi.mocked(open).mockResolvedValue(null);
    await runCommand();
    await vi.waitFor(() => expect(useLayout.getState().module).toBe("raw-review"));
    expect(useLayout.getState().rawReviewTarget).toBeNull();
  });
});
