import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
afterEach(checkedCleanup());
beforeEach(() => {
  useCommandStore.setState({ commands: [] });
  usePalette.setState({ open: false, initial: "", recent: [] });
  useLayout.setState({ module: "library", aiSection: "overview" });
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

describe("comandi AI nella palette", () => {
  it.each([
    ["AI: Panoramica", "overview"],
    ["AI: Rianalisi", "reprocess"],
    ["AI: Pulisci etichette", "cleanup"],
    ["AI: Mappa", "map"],
  ] as const)("«%s» apre la sezione %s", async (label, section) => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <Shell />
      </QueryClientProvider>,
    );
    const user = userEvent.setup();
    await user.keyboard("{Control>}k{/Control}");
    const input = await screen.findByRole("combobox");
    await user.type(input, label);
    await user.click(await screen.findByRole("option", { name: new RegExp(`^${label}`) }));
    expect(useLayout.getState().module).toBe("ai");
    expect(useLayout.getState().aiSection).toBe(section);
  });
});
