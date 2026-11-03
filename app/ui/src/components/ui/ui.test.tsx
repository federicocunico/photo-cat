// Componenti condivisi: ruoli, focus, Esc, ritorno del focus e Invio (ui-ux-overhaul §1.3).
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import "../../i18n";
import { Icon } from "../icons";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  ContextMenu,
  DataTable,
  Dialog,
  Disclosure,
  EmptyState,
  FolderPicker,
  FormField,
  IconButton,
  InlineAlert,
  Kbd,
  Menu,
  PageHeader,
  Popover,
  SegmentedControl,
  Tabs,
  Tooltip,
} from ".";

afterEach(cleanup);

describe("Button e IconButton", () => {
  it("esegue con Invio e blocca i clic ripetuti durante un'azione asincrona", async () => {
    const user = userEvent.setup();
    let resolve!: () => void;
    const onClick = vi.fn(() => new Promise<void>((r) => (resolve = r)));
    render(<Button onClick={onClick}>Salva</Button>);
    const b = screen.getByRole("button", { name: "Salva" });
    b.focus();
    await user.keyboard("{Enter}");
    await user.click(b);
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(b).toHaveAttribute("aria-busy", "true");
    await act(async () => resolve());
    await waitFor(() => expect(b).not.toHaveAttribute("aria-busy"));
  });

  it("un comando non disponibile resta raggiungibile e dice perché", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <Button disabledReason="nessuna foto selezionata" onClick={onClick}>
        Esporta
      </Button>,
    );
    const b = screen.getByRole("button", { name: /Esporta/ });
    expect(b).not.toBeDisabled();
    expect(b).toHaveAttribute("aria-disabled", "true");
    expect(b).toHaveAccessibleDescription("Non disponibile: nessuna foto selezionata");
    await user.click(b);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("IconButton ha sempre un nome accessibile", () => {
    render(<IconButton label="Chiudi pannello" icon={<Icon.Close />} />);
    expect(screen.getByRole("button", { name: "Chiudi pannello" })).toBeInTheDocument();
  });
});

describe("impaginazione e stato", () => {
  it("PageHeader e Card espongono i titoli come intestazioni", () => {
    render(
      <>
        <PageHeader title="Importa" actions={<Button>Aggiungi</Button>} />
        <Card title="Cartelle">contenuto</Card>
      </>,
    );
    expect(screen.getByRole("heading", { level: 1, name: "Importa" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Cartelle" })).toBeInTheDocument();
  });

  it("InlineAlert: errori annunciati subito, il resto con cortesia; si chiude", async () => {
    const user = userEvent.setup();
    const onDismiss = vi.fn();
    render(
      <>
        <InlineAlert tone="danger" title="Errore">
          Cartella non trovata
        </InlineAlert>
        <InlineAlert tone="ok" onDismiss={onDismiss}>
          Fatto
        </InlineAlert>
      </>,
    );
    expect(screen.getByRole("alert")).toHaveTextContent("Cartella non trovata");
    expect(screen.getByRole("status")).toHaveTextContent("Fatto");
    await user.click(screen.getByRole("button", { name: "Chiudi l'avviso" }));
    expect(onDismiss).toHaveBeenCalled();
  });

  it("EmptyState, Badge e Kbd", () => {
    render(
      <>
        <EmptyState title="Nessuna foto" action={<Button>Importa una cartella</Button>} />
        <Badge label="3 persone da rivedere">3</Badge>
        <Kbd keys={["Ctrl", "K"]} />
      </>,
    );
    expect(screen.getByRole("heading", { name: "Nessuna foto" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "3 persone da rivedere" })).toBeInTheDocument();
    expect(document.querySelectorAll("kbd")).toHaveLength(2);
  });
});

describe("FormField", () => {
  it("collega etichetta, aiuto ed errore al controllo", () => {
    render(
      <FormField label="Nome" hint="Come compare nei filtri" error="Obbligatorio" required>
        <input className="field" />
      </FormField>,
    );
    const input = screen.getByRole("textbox", { name: /Nome/ });
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAttribute("aria-required", "true");
    expect(input).toHaveAccessibleDescription("Come compare nei filtri Obbligatorio");
  });
});

describe("SegmentedControl, Tabs, Disclosure", () => {
  it("SegmentedControl: frecce per spostarsi, una scelta resta sempre attiva", async () => {
    const user = userEvent.setup();
    function Host() {
      const [v, setV] = useState<"grid" | "list">("grid");
      return (
        <SegmentedControl
          label="Vista"
          value={v}
          onChange={setV}
          options={[
            { value: "grid", label: "Griglia" },
            { value: "list", label: "Elenco" },
          ]}
        />
      );
    }
    render(<Host />);
    expect(screen.getByRole("radiogroup", { name: "Vista" })).toBeInTheDocument();
    const grid = screen.getByRole("radio", { name: "Griglia" });
    expect(grid).toHaveAttribute("aria-checked", "true");
    await user.click(grid);
    expect(grid).toHaveAttribute("aria-checked", "true");
    grid.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "Elenco" })).toHaveFocus();
    await user.keyboard(" ");
    expect(screen.getByRole("radio", { name: "Elenco" })).toHaveAttribute("aria-checked", "true");
  });

  it("Tabs: ruoli tablist/tab/tabpanel e frecce", async () => {
    const user = userEvent.setup();
    function Host() {
      const [v, setV] = useState("a");
      return (
        <Tabs
          label="Sezioni"
          value={v}
          onChange={setV}
          tabs={[
            { value: "a", label: "Generale", content: "pannello A" },
            { value: "b", label: "Avanzate", content: "pannello B" },
          ]}
        />
      );
    }
    render(<Host />);
    expect(screen.getByRole("tablist", { name: "Sezioni" })).toBeInTheDocument();
    screen.getByRole("tab", { name: "Generale" }).focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Avanzate" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveTextContent("pannello B");
  });

  it("Disclosure: il titolo è un pulsante con aria-expanded e lo stato si può persistere", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <Disclosure title="Luogo" defaultOpen={false} onOpenChange={onOpenChange}>
        Giappone
      </Disclosure>,
    );
    const trigger = screen.getByRole("button", { name: "Luogo" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    trigger.focus();
    await user.keyboard("{Enter}");
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Giappone")).toBeVisible();
    expect(onOpenChange).toHaveBeenCalledWith(true);
  });
});

describe("Tooltip", () => {
  it("compare al focus e si chiude con Esc", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Ricarica la cartella">
        <button type="button">Aggiorna</button>
      </Tooltip>,
    );
    await user.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Ricarica la cartella");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("tooltip")).not.toBeInTheDocument());
  });
});

describe("Menu, ContextMenu, Popover", () => {
  it("Menu: si apre da tastiera, Invio esegue, Esc chiude e il focus torna al pulsante", async () => {
    const user = userEvent.setup();
    const rename = vi.fn();
    render(
      <Menu
        trigger={<button type="button">Altro</button>}
        items={[
          { label: "Rinomina", onSelect: rename, shortcut: "F2" },
          { kind: "separator" },
          { label: "Rimuovi", danger: true, onSelect: () => {} },
        ]}
      />,
    );
    const trigger = screen.getByRole("button", { name: "Altro" });
    trigger.focus();
    await user.keyboard("{Enter}");
    expect(await screen.findByRole("menu", { name: "Altro" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("menuitem", { name: /Rinomina/ })).toHaveFocus());
    await user.keyboard("{Enter}");
    expect(rename).toHaveBeenCalled();
    await waitFor(() => expect(trigger).toHaveFocus());

    await user.keyboard("{Enter}");
    await screen.findByRole("menu");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("ContextMenu: si apre con il clic destro", async () => {
    const select = vi.fn();
    render(
      <ContextMenu label="Foto" items={[{ label: "Apri con…", onSelect: select }]}>
        <div>miniatura</div>
      </ContextMenu>,
    );
    fireEvent.contextMenu(screen.getByText("miniatura"));
    const item = await screen.findByRole("menuitem", { name: "Apri con…" });
    fireEvent.click(item);
    expect(select).toHaveBeenCalled();
  });

  it("Popover: dialogo non modale con nome, Esc chiude e il focus torna", async () => {
    const user = userEvent.setup();
    render(
      <Popover label="Scegli un tag" trigger={<button type="button">Tag</button>}>
        <input aria-label="Cerca tag" />
      </Popover>,
    );
    const trigger = screen.getByRole("button", { name: "Tag" });
    await user.click(trigger);
    expect(await screen.findByRole("dialog", { name: "Scegli un tag" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Cerca tag" })).toHaveFocus());
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});

describe("Dialog e ConfirmDialog", () => {
  function DialogHost() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button type="button" onClick={() => setOpen(true)}>
          Nuovo catalogo
        </button>
        <Dialog
          open={open}
          onOpenChange={setOpen}
          title="Nuovo catalogo"
          footer={<Button variant="primary">Crea</Button>}
        >
          <input aria-label="Nome del catalogo" />
        </Dialog>
      </>
    );
  }

  it("Dialog: modale con titolo, focus dentro, Esc chiude e il focus torna", async () => {
    const user = userEvent.setup();
    render(<DialogHost />);
    const opener = screen.getByRole("button", { name: "Nuovo catalogo" });
    await user.click(opener);
    const dialog = await screen.findByRole("dialog", { name: "Nuovo catalogo" });
    expect(dialog.contains(document.activeElement)).toBe(true);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(opener).toHaveFocus());
  });

  it("ConfirmDialog: verbo esplicito, conteggio, effetto sui file; il focus parte da Annulla", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn(async () => {});
    function Host() {
      const [open, setOpen] = useState(true);
      return (
        <ConfirmDialog
          open={open}
          onOpenChange={setOpen}
          title="Eliminare il catalogo «Viaggi»?"
          confirmLabel="Elimina catalogo"
          count={3}
          files="untouched"
          onConfirm={onConfirm}
        />
      );
    }
    render(<Host />);
    const dialog = await screen.findByRole("alertdialog", { name: /Viaggi/ });
    expect(dialog).toHaveTextContent("3 elementi");
    expect(dialog).toHaveTextContent("I file sul disco non vengono toccati.");
    expect(screen.getByRole("button", { name: "Annulla" })).toHaveFocus();
    await user.click(screen.getByRole("button", { name: "Elimina catalogo" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("ConfirmDialog: Esc annulla senza confermare", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <ConfirmDialog
        open
        onOpenChange={onOpenChange}
        title="Sposta i file?"
        confirmLabel="Sposta 12 file"
        files="affected"
        onConfirm={onConfirm}
      />,
    );
    await screen.findByRole("alertdialog");
    await user.keyboard("{Escape}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onConfirm).not.toHaveBeenCalled();
  });
});

describe("FolderPicker", () => {
  it("mostra il percorso, lo modifica a mano e con Scegli…", async () => {
    const user = userEvent.setup();
    function Host() {
      const [p, setP] = useState("");
      return (
        <FolderPicker
          label="Cartella delle foto"
          value={p}
          onChange={setP}
          pick={async () => "D:\\Foto\\2024"}
        />
      );
    }
    render(<Host />);
    const input = screen.getByRole("textbox", { name: "Cartella delle foto" });
    expect(input).toHaveAttribute("placeholder", "Nessuna cartella scelta");
    await user.click(screen.getByRole("button", { name: "Scegli…" }));
    await waitFor(() => expect(input).toHaveValue("D:\\Foto\\2024"));
  });

  it("accetta una cartella trascinata", () => {
    const onChange = vi.fn();
    render(<FolderPicker label="Cartella" value="" onChange={onChange} />);
    const zone = screen.getByRole("textbox", { name: "Cartella" }).closest(".ui-folder")!;
    fireEvent.drop(zone, {
      dataTransfer: { files: [{ path: "E:\\Viaggi" }], getData: () => "" },
    });
    expect(onChange).toHaveBeenCalledWith("E:\\Viaggi");
  });
});

describe("DataTable", () => {
  const rows = [
    { id: 1, code: "DECODE_FAILED", n: 12 },
    { id: 2, code: "IO_ERROR", n: 3 },
    { id: 3, code: "TIMEOUT", n: 1 },
  ];
  it("un solo tabstop, frecce/Home/Fine spostano la selezione, Invio apre", async () => {
    const user = userEvent.setup();
    const onActivate = vi.fn();
    function Host() {
      const [sel, setSel] = useState<number | null>(null);
      return (
        <DataTable
          label="Errori"
          columns={[
            { key: "code", header: "Codice", cell: (r) => r.code },
            { key: "n", header: "File", cell: (r) => r.n, align: "end" },
          ]}
          rows={rows}
          rowId={(r) => r.id}
          selected={sel}
          onSelect={(r) => setSel(r.id)}
          onActivate={onActivate}
        />
      );
    }
    render(<Host />);
    expect(screen.getByRole("table", { name: "Errori" })).toBeInTheDocument();
    const body = screen.getAllByRole("row").slice(1);
    expect(body.filter((r) => r.tabIndex === 0)).toHaveLength(1);
    await user.tab();
    expect(body[0]).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getAllByRole("row")[2]).toHaveFocus();
    expect(screen.getAllByRole("row")[2]).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{End}");
    expect(screen.getAllByRole("row")[3]).toHaveFocus();
    await user.keyboard("{Home}{Enter}");
    expect(onActivate).toHaveBeenCalledWith(rows[0]);
  });
});
