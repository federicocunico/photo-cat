import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { useLayout } from "../../store/layout";
import { emptyFilter, idsSelection, useLibrary } from "../../store/library";
import { usePalette } from "../../store/palette";
import { useCommandStore } from "../../commands/registry";
import { ModuleNav } from "../shell/ModuleNav";
import { LibraryHeader } from "../../modules/library/LibraryHeader";
import { SelectionBar } from "../../modules/library/SelectionBar";
import { CommandPalette } from "./CommandPalette";
import { usePaletteCommand } from "../../commands/paletteCommand";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup());
beforeEach(() => {
  vi.clearAllMocks();
  useCommandStore.setState({ commands: [] });
  usePalette.setState({ open: false, initial: "", recent: [] });
  useLayout.setState({ module: "library" });
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
});

function Shell({ children }: { children?: React.ReactNode }) {
  usePaletteCommand();
  return (
    <>
      <ModuleNav />
      {children}
      <CommandPalette />
    </>
  );
}

function renderShell(children?: React.ReactNode) {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <Shell>{children}</Shell>
    </QueryClientProvider>,
  );
}

async function openAndType(text: string) {
  const user = userEvent.setup();
  await user.keyboard("{Control>}k{/Control}");
  const input = await screen.findByRole("combobox");
  if (text) await user.type(input, text);
  return { user, input };
}

describe("palette dei comandi", () => {
  it("Vai a un modulo: Ctrl+K, «pers», Invio su Vai a Persone", async () => {
    renderShell();
    const { user } = await openAndType("pers");
    expect(await screen.findByRole("option", { name: /Vai a Persone/ })).toBeInTheDocument();
    await user.keyboard("{Enter}");
    expect(useLayout.getState().module).toBe("people");
    expect(usePalette.getState().open).toBe(false);
  });

  it("i comandi mostrano la scorciatoia; usati di recente compaiono a palette vuota", async () => {
    renderShell();
    const { user } = await openAndType("catalog");
    const go = await screen.findByRole("option", { name: /Vai a Cataloghi/ });
    expect(go).toHaveTextContent(/Ctrl\+3/);
    await user.keyboard("{Enter}");
    await openAndType("");
    const recent = await screen.findByRole("group", { name: "Recenti" });
    expect(within(recent).getByRole("option", { name: /Vai a Cataloghi/ })).toBeInTheDocument();
  });

  it("Nessun risultato: lo dice e propone la ricerca nei nomi dei file", async () => {
    // Il backend restituisce solo i file che corrispondono: qui nessuno.
    commandsMock.libraryPage.mockResolvedValue({ items: [], next: null } as never);
    renderShell();
    const { user } = await openAndType("zzqqzz");
    expect(await screen.findByText("Nessun risultato")).toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: "Cerca «zzqqzz» nei nomi dei file" }),
    ).toBeInTheDocument();
    await user.keyboard("{Enter}");
    expect(useLibrary.getState().filter.text).toBe("zzqqzz");
    expect(useLayout.getState().module).toBe("library");
  });

  it("Filtro per persona: Libreria filtrata con il chip della persona", async () => {
    commandsMock.listPeople.mockResolvedValue([
      { id: 12, name: "Giulia Rossi", identityUid: null, photos: 40, faces: 60, cover: null },
    ] as never);
    renderShell(<LibraryHeader />);
    const { user } = await openAndType("giulia");
    await user.click(await screen.findByRole("option", { name: "Giulia Rossi" }));
    expect(useLibrary.getState().filter.personIds).toEqual([12]);
    expect(
      await screen.findByRole("button", { name: "Rimuovi filtro Giulia Rossi" }),
    ).toBeInTheDocument();
  });

  it("Azione sulla selezione: «Aggiungi tag…» apre il selettore dei tag per le 5 foto", async () => {
    renderShell(<SelectionBar />);
    // Senza selezione l'azione non c'è.
    await openAndType("aggiungi tag");
    expect(screen.queryByRole("option", { name: "Aggiungi tag…" })).not.toBeInTheDocument();
    act(() => usePalette.getState().close());
    act(() => useLibrary.setState({ selection: idsSelection([1, 2, 3, 4, 5]) }));
    const { user } = await openAndType("aggiungi tag");
    await user.click(await screen.findByRole("option", { name: "Aggiungi tag…" }));
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Cerca o scrivi un tag" })).toBeInTheDocument(),
    );
  });
});
