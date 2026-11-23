// Barra dei menu (application-menu, in-app-guidance "Menu Aiuto"; ui-usability-audit 4.1–4.5).
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { useAppMenuCommands } from "../../commands/appMenu";
import { useGridShortcuts, useActivePane, type ActivePane } from "../../commands/gridShortcuts";
import { usePaletteCommand } from "../../commands/paletteCommand";
import { useSheetCommand } from "../../commands/sheetCommand";
import {
  availableCommands,
  latestCommands,
  menuEntries,
  shortcutText,
  useCommandStore,
  type MenuId,
} from "../../commands/registry";
import { useUndoHotkey } from "../../store/undo";
import { useGuide } from "../../store/guide";
import { useLayout } from "../../store/layout";
import { emptyFilter, idsSelection, useLibrary } from "../../store/library";
import { useAboutDialog, useMenuIntent } from "../../store/menuIntent";
import { AboutDialog } from "../about/AboutDialog";
import { ModuleNav } from "./ModuleNav";
import { Panels } from "./Panels";
import { TopBar } from "./TopBar";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
const windowMock = vi.hoisted(() => ({
  close: vi.fn(async () => {}),
  setFullscreen: vi.fn(async () => {}),
  isFullscreen: vi.fn(async () => false),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => windowMock }));

beforeEach(() => {
  useCommandStore.setState({ commands: [] });
  useLayout.setState({ module: "library", navCompact: false, theme: "dark" });
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection([]) });
  useMenuIntent.setState({ intent: null });
  useAboutDialog.setState({ open: false });
  useGuide.setState({ intro: false, report: false });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

function Pane({ pane }: { pane: ActivePane }) {
  const ref = useRef<HTMLDivElement>(null);
  useActivePane(pane, ref);
  return (
    <div ref={ref} tabIndex={0} data-testid="pane">
      volti
    </div>
  );
}

function Shell({ library = false, pane }: { library?: boolean; pane?: ActivePane }) {
  useAppMenuCommands();
  useSheetCommand();
  usePaletteCommand();
  useUndoHotkey();
  useGridShortcuts();
  return (
    <>
      <TopBar />
      <ModuleNav />
      {library && <Panels left={<div>sx</div>} center={<div>centro</div>} right={<div>dx</div>} />}
      {pane && <Pane pane={pane} />}
      <input aria-label="Campo" />
      <AboutDialog />
    </>
  );
}

const renderShell = (props: { library?: boolean; pane?: ActivePane } = {}) =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Shell {...props} />
    </QueryClientProvider>,
  );

const menuNames = ["File", "Modifica", "Vista", "Vai", "Strumenti", "Aiuto"];

async function openMenu(name: string) {
  const user = userEvent.setup();
  await user.click(screen.getByRole("menuitem", { name }));
  return { user, menu: await screen.findByRole("menu") };
}

const items = (menu: HTMLElement) =>
  within(menu)
    .queryAllByRole("menuitem")
    .map((i) => i.textContent ?? "");

describe("barra dei menu", () => {
  it("Menu sempre presenti: File, Modifica, Vista, Vai, Strumenti, Aiuto, senza il pulsante «?»", () => {
    renderShell();
    const bar = screen.getByRole("menubar", { name: "Barra dei menu" });
    expect(
      within(bar)
        .getAllByRole("menuitem")
        .map((m) => m.textContent),
    ).toEqual(menuNames);
    expect(screen.queryByRole("button", { name: /^Aiuto/ })).toBeNull();
  });

  it("4.1: ogni voce richiesta esiste nel registro, con lo stesso nome della palette", () => {
    renderShell({ library: true });
    const required: Record<MenuId, string[]> = {
      file: [
        "Importa una cartella…",
        "Esporta…",
        "Apri una cartella di RAW…",
        "Apri un progetto FujiView…",
        "Mostra la cartella dei dati",
        "Impostazioni",
        "Esci",
      ],
      edit: [
        "Annulla operazione",
        "Seleziona tutto",
        "Inverti selezione",
        "Deseleziona",
        "Tag del set attivo",
        "Valutazione",
        "Aggiungi a un catalogo…",
        "Sposta nel Cestino",
      ],
      view: [
        "Navigazione compatta",
        "Pannelli laterali",
        "Inspector",
        "Dimensione delle miniature",
        "Raggruppa foto simili",
        "Mappa delle foto",
        "Tema",
        "Schermo intero",
      ],
      go: ["Libreria", "Persone", "Varianti RAW", "Attività", "Cerca comandi, persone, tag…"],
      tools: [
        "Rianalizza le foto…",
        "Metti da parte i duplicati",
        "Controlla la qualità",
        "Modelli AI",
        "Diagnostica e registri",
      ],
      help: [
        "Guida",
        "Scorciatoie da tastiera",
        "Primi passi",
        "Rivedi l'introduzione",
        "Mostra di nuovo i suggerimenti",
        "Segnala un problema",
        "Copia le informazioni di sistema",
        "Informazioni su photo-cat",
      ],
    };
    for (const [menu, labels] of Object.entries(required) as Array<[MenuId, string[]]>) {
      const shown = menuEntries(menu)
        .flat()
        .map((e) => e.placement.submenu ?? e.command.label);
      for (const label of labels) expect(shown, `${menu}: ${label}`).toContain(label);
    }
    // Stesso nome e stessa scorciatoia nella palette.
    const palette = new Map(availableCommands().map((c) => [c.id, c]));
    for (const c of latestCommands().filter((c) => c.menu && palette.has(c.id)))
      expect(palette.get(c.id)!.label).toBe(c.label);
  });

  it("Stessa scorciatoia ovunque: Vista › Inspector mostra Ctrl+I", async () => {
    renderShell({ library: true });
    const { menu } = await openMenu("Vista");
    const inspector = within(menu).getByRole("menuitemcheckbox", { name: /^Inspector/ });
    expect(inspector).toHaveTextContent("Ctrl+I");
    expect(inspector).toHaveAttribute("aria-checked", "true");
  });

  it("Comando non disponibile: Sposta nel Cestino è disattivata e dice perché", async () => {
    renderShell();
    const { user, menu } = await openMenu("Modifica");
    const trash = within(menu).getByRole("menuitem", { name: /^Sposta nel Cestino/ });
    expect(trash).toHaveAttribute("aria-disabled", "true");
    expect(trash).toHaveAccessibleDescription("Seleziona prima delle foto in Libreria");
    await user.click(trash);
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(useLayout.getState().module).toBe("library");
  });

  it("Importare dal menu: File › Importa una cartella… apre Importa e il suo selettore", async () => {
    renderShell();
    useLayout.setState({ module: "people" });
    const { user, menu } = await openMenu("File");
    await user.click(within(menu).getByRole("menuitem", { name: /^Importa una cartella…/ }));
    expect(useLayout.getState().module).toBe("import");
    expect(useMenuIntent.getState().intent).toBe("import.pick");
  });

  it("Selezione dal menu: Modifica › Seleziona tutto agisce sul riquadro attivo come Ctrl+A", async () => {
    const pane: ActivePane = {
      selectAll: vi.fn(),
      invert: vi.fn(),
      clear: vi.fn(),
      count: () => 0,
    };
    renderShell({ pane });
    fireEvent.mouseDown(screen.getByTestId("pane"));
    const { user, menu } = await openMenu("Modifica");
    await user.click(within(menu).getByRole("menuitem", { name: /^Seleziona tutto/ }));
    expect(pane.selectAll).toHaveBeenCalledTimes(1);
  });

  it("Aprire un modulo dal menu: Vai › Persone con la stessa scorciatoia della navigazione", async () => {
    renderShell();
    const { user, menu } = await openMenu("Vai");
    const people = within(menu).getByRole("menuitem", { name: /^Persone/ });
    expect(people).toHaveTextContent("Ctrl+4");
    await user.click(people);
    expect(useLayout.getState().module).toBe("people");
  });

  it("Ctrl+O e Ctrl+E: Importa ed Esporta, senza conflitti con altri comandi", () => {
    renderShell({ library: true });
    const owners = (text: string) =>
      latestCommands()
        .filter((c) => c.shortcut && !c.fallbackFor && shortcutText(c.shortcut) === text)
        .map((c) => c.id);
    expect(owners("Ctrl+O")).toEqual(["file.import"]);
    expect(owners("Ctrl+E")).toEqual(["file.export"]);
    fireEvent.keyDown(window, { key: "o", ctrlKey: true });
    expect(useLayout.getState().module).toBe("import");
    fireEvent.keyDown(window, { key: "e", ctrlKey: true });
    expect(useLayout.getState().module).toBe("organize");
  });

  it("Tema dal menu Vista: sottomenu con la scelta attiva spuntata", async () => {
    renderShell();
    const { user, menu } = await openMenu("Vista");
    await user.click(within(menu).getByRole("menuitem", { name: /^Tema/ }));
    const light = await screen.findByRole("menuitemcheckbox", { name: /^Chiaro/ });
    expect(screen.getByRole("menuitemcheckbox", { name: /^Scuro/ })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(light);
    expect(useLayout.getState().theme).toBe("light");
  });
});

describe("menu Aiuto", () => {
  it("contiene tutte le voci della guida, nell'ordine", async () => {
    renderShell();
    const { menu } = await openMenu("Aiuto");
    expect(items(menu).map((s) => s.replace(/\?$/, ""))).toEqual([
      "Guida",
      "Scorciatoie da tastiera",
      "Primi passi",
      "Rivedi l'introduzione",
      "Mostra di nuovo i suggerimenti",
      "Segnala un problema",
      "Copia le informazioni di sistema",
      "Informazioni su photo-cat",
    ]);
  });

  it("Informazioni dal menu: si apre la finestra con versione e build", async () => {
    renderShell();
    const { user, menu } = await openMenu("Aiuto");
    await user.click(within(menu).getByRole("menuitem", { name: /^Informazioni su photo-cat/ }));
    const dialog = await screen.findByRole("dialog", { name: "Informazioni su photo-cat" });
    expect(await within(dialog).findByText("photo-cat 0.5.1")).toBeInTheDocument();
    expect(within(dialog).getByText(/^a1b2c3d · .+ · release$/)).toBeInTheDocument();
  });

  it("Segnalare un problema: apre Diagnostica con l'esportazione in evidenza", async () => {
    renderShell();
    const { user, menu } = await openMenu("Aiuto");
    await user.click(within(menu).getByRole("menuitem", { name: /^Segnala un problema/ }));
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
    });
    expect(useGuide.getState().report).toBe(true);
  });

  it("Rivedi l'introduzione e Mostra di nuovo i suggerimenti", async () => {
    renderShell();
    let { user, menu } = await openMenu("Aiuto");
    await user.click(within(menu).getByRole("menuitem", { name: /^Rivedi l'introduzione/ }));
    expect(useGuide.getState().intro).toBe(true);
    ({ user, menu } = await openMenu("Aiuto"));
    await user.click(
      within(menu).getByRole("menuitem", { name: /^Mostra di nuovo i suggerimenti/ }),
    );
    expect(useGuide.getState().seen).toEqual([]);
  });
});

describe("barra dei menu da tastiera (4.4)", () => {
  const trigger = (name: string) => screen.getByRole("menuitem", { name });

  it("Alt premuto e rilasciato porta il focus sul primo menu; frecce e Invio aprono Vista", async () => {
    renderShell();
    const user = userEvent.setup();
    const field = screen.getByRole("button", { name: "Riduci la navigazione" });
    field.focus();
    fireEvent.keyDown(field, { key: "Alt" });
    fireEvent.keyUp(field, { key: "Alt" });
    expect(trigger("File")).toHaveFocus();
    expect(screen.getByRole("menubar")).toHaveAttribute("data-keyboard");
    await user.keyboard("{ArrowRight}{ArrowRight}{Enter}");
    const menu = await screen.findByRole("menu");
    expect(trigger("Vista")).toHaveAttribute("aria-expanded", "true");
    await waitFor(() => expect(menu.querySelector('[role^="menuitem"]')).toHaveFocus());
  });

  it("Esc due volte chiude il menu e riporta il focus dov'era", async () => {
    renderShell();
    const user = userEvent.setup();
    const before = screen.getByRole("button", { name: "Riduci la navigazione" });
    before.focus();
    fireEvent.keyDown(window, { key: "F10" });
    expect(trigger("File")).toHaveFocus();
    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(trigger("File")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(before).toHaveFocus();
  });

  it("Alt+1 non apre il menu (è la valutazione a una stella); nei campi Alt non attiva la barra", () => {
    renderShell();
    const field = screen.getByRole("textbox", { name: "Campo" });
    field.focus();
    fireEvent.keyDown(field, { key: "Alt" });
    fireEvent.keyUp(field, { key: "Alt" });
    expect(field).toHaveFocus();
    const nav = screen.getByRole("button", { name: "Riduci la navigazione" });
    nav.focus();
    fireEvent.keyDown(nav, { key: "Alt" });
    fireEvent.keyDown(nav, { key: "1", code: "Digit1", altKey: true });
    fireEvent.keyUp(nav, { key: "Alt" });
    expect(nav).toHaveFocus();
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("lettere d'accesso: con la barra attiva «u» apre Aiuto", async () => {
    renderShell();
    fireEvent.keyDown(window, { key: "F10" });
    act(() => {
      fireEvent.keyDown(trigger("File"), { key: "u" });
    });
    expect(await screen.findByRole("menu")).toBeInTheDocument();
    expect(trigger("Aiuto")).toHaveAttribute("aria-expanded", "true");
  });
});

describe("menu nativo di macOS (4.6)", () => {
  it("le voci vanno al menu nativo con acceleratori, disponibilità e spunte", async () => {
    const { nativeMenuItems, toAccelerator } = await import("./nativeMenu");
    renderShell({ library: true });
    const items = nativeMenuItems();
    const byId = new Map(items.map((i) => [i.id, i]));
    expect(byId.get("file.import")).toMatchObject({
      menu: "file",
      accelerator: "CmdOrCtrl+O",
      enabled: true,
    });
    expect(byId.get("edit.moveToTrash")).toMatchObject({ enabled: false });
    expect(byId.get("panels.inspector")).toMatchObject({
      checked: true,
      accelerator: "CmdOrCtrl+I",
    });
    expect(toAccelerator({ key: "i", mod: true, shift: true })).toBe("CmdOrCtrl+Shift+I");
  });

  it("menu://command esegue il comando con quell'id; una voce disattivata non fa nulla", async () => {
    const { emit } = await import("../../test/ipcMock");
    const { useNativeMenu } = await import("./nativeMenu");
    function Native() {
      useNativeMenu(false);
      return null;
    }
    render(
      <QueryClientProvider client={new QueryClient()}>
        <Shell />
        <Native />
      </QueryClientProvider>,
    );
    act(() => emit("menuCommand", "file.export"));
    expect(useLayout.getState().module).toBe("organize");
    act(() => emit("menuCommand", "edit.moveToTrash"));
    expect(useLayout.getState().module).toBe("organize");
  });
});
