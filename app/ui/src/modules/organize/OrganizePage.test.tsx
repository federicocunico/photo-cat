import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { emptyFilter, idsSelection, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useSequence } from "../../store/sequence";
import { useToast } from "../../store/toasts";
import { ToastHost } from "../../components/feedback/ToastHost";
import { OrganizePage } from "./OrganizePage";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
beforeEach(() => {
  vi.clearAllMocks();
  useToast.setState({ notices: [] });
  useLibrary.setState({ filter: emptyFilter(), selection: idsSelection() });
  useLayout.setState({ module: "organize", jobFocus: null });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

const renderPage = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <OrganizePage />
      <ToastHost />
    </QueryClientProvider>,
  );

const button = (name: string | RegExp) => screen.getByRole("button", { name });

it("sezioni Esporta dati, Organizza in cartelle e Sequenza", async () => {
  renderPage();
  for (const name of ["Esporta dati", "Organizza in cartelle", "Sequenza"])
    expect(await screen.findByRole("heading", { level: 2, name })).toBeInTheDocument();
});

it("senza selezione l'ambito predefinito è la Libreria filtrata", async () => {
  renderPage();
  expect(await screen.findByLabelText("Ambito")).toHaveValue("filter");
  expect(screen.getByRole("option", { name: "Libreria filtrata" })).toBeInTheDocument();
});

it("con una selezione l'ambito è 'Selezione (N foto)'; a zero foto lo spiega", async () => {
  useLibrary.setState({ selection: idsSelection(Array.from({ length: 40 }, (_, i) => i + 1)) });
  renderPage();
  const scope = await screen.findByLabelText("Ambito");
  expect(scope).toHaveValue("selection");
  expect(screen.getByRole("option", { name: "Selezione (40 foto)" })).toBeInTheDocument();

  useLibrary.setState({ selection: idsSelection() });
  await waitFor(() =>
    expect(screen.getByRole("status")).toHaveTextContent(
      "Nessuna foto selezionata: seleziona foto in Libreria o scegli un altro ambito",
    ),
  );
  const start = button("Esporta");
  expect(start).toHaveAttribute("aria-disabled", "true");
  expect(start).toHaveAccessibleDescription(/Nessuna foto selezionata/);
});

it("esporta in CSV con separatore, BOM e campi scelti; l'avviso porta all'attività", async () => {
  renderPage();
  fireEvent.change(await screen.findByLabelText("Ambito"), { target: { value: "library" } });
  await screen.findByLabelText("Persone");
  fireEvent.click(screen.getByLabelText("Persone"));
  fireEvent.change(screen.getByLabelText("File di destinazione"), {
    target: { value: "D:\\Export\\catalogo.csv" },
  });
  // Separatore e BOM stanno in "Avanzate".
  expect(screen.queryByLabelText("Separatore di colonna")).toBeNull();
  fireEvent.click(button("Avanzate"));
  expect(screen.getByLabelText("Separatore di colonna")).toHaveValue(";");
  fireEvent.click(button("Esporta"));
  await waitFor(() =>
    expect(commandsMock.startExport).toHaveBeenCalledWith({
      scope: { scope: "library" },
      destination: "D:\\Export\\catalogo.csv",
      format: "csv",
      separator: ";",
      bom: true,
      fields: ["path", "tags"],
    }),
  );
  expect(await screen.findByText("Esportazione dei dati avviata")).toBeInTheDocument();
  fireEvent.click(button("Segui l'attività"));
  expect(useLayout.getState()).toMatchObject({ module: "jobs", jobFocus: 1 });
});

it("passi Anteprima → Verifica → Crea con lo stato; Crea chiede conferma sui file", async () => {
  commandsMock.organizePreview.mockResolvedValue({
    entries: [],
    folders: { "Kyoto/2025": 120, "Senza luogo/2025": 3 },
    files: 123,
    bytes: 3 * 1024 * 1024 * 1024,
    conflicts: 2,
  } as never);
  commandsMock.organizePreflight
    .mockResolvedValueOnce({
      available: false,
      alternative: "hardlink",
      developerMode: false,
      reason: "symlink_privilege",
      message: "",
    } as never)
    .mockResolvedValue({
      available: true,
      alternative: "hardlink",
      developerMode: false,
      reason: null,
      message: "",
    } as never);
  commandsMock.startOrganize.mockResolvedValue(9 as never);
  renderPage();
  fireEvent.change(await screen.findByLabelText("Ambito"), { target: { value: "library" } });
  fireEvent.change(screen.getByLabelText("Cartella di destinazione"), {
    target: { value: "E:\\Organizzate" },
  });
  fireEvent.change(screen.getByLabelText("Metodo"), { target: { value: "symlink" } });
  const steps = screen.getByRole("list", { name: "Passi per creare le cartelle" });
  const items = () => [...steps.children] as HTMLElement[];
  expect(items()[0]).toHaveTextContent("Anteprima");
  expect(items()[0]).toHaveTextContent("Da fare");
  expect(items()[2]).toHaveTextContent("Si abilita al termine dei passi 1 e 2");
  expect(button("Crea cartelle")).toHaveAttribute("aria-disabled", "true");
  expect(button("Crea cartelle")).toHaveAccessibleDescription(
    /Completa prima l'anteprima e la verifica del metodo/,
  );

  fireEvent.click(button("Anteprima"));
  expect(await screen.findByText(/123 file · 2 conflitti/)).toBeInTheDocument();
  expect(screen.getByText("Kyoto/2025 · 120")).toBeInTheDocument();
  expect(screen.getByText("Senza luogo/2025 · 3")).toBeInTheDocument();
  expect(items()[0]).toHaveTextContent("Fatto");
  expect(commandsMock.organizePreview).toHaveBeenCalledWith(
    expect.objectContaining({ levels: ["place", "year"], method: "symlink" }),
  );

  fireEvent.click(button("Verifica metodo"));
  expect(await screen.findByText(/Modalità sviluppatore di Windows/)).toBeInTheDocument();
  expect(items()[1]).toHaveTextContent("Non superata");
  fireEvent.click(await screen.findByRole("button", { name: "Collegamento fisso (hard link)" }));
  expect(screen.getByLabelText("Metodo")).toHaveValue("hardlink");
  // Cambiare metodo invalida anteprima e verifica: vanno ripetute.
  expect(items()[0]).toHaveTextContent("Da ripetere");
  expect(button("Crea cartelle")).toHaveAttribute("aria-disabled", "true");
  fireEvent.click(button("Anteprima"));
  await screen.findByText(/123 file/);
  fireEvent.click(button("Verifica metodo"));
  await waitFor(() => expect(button("Crea cartelle")).not.toHaveAttribute("aria-disabled"));
  expect(items()[2]).toHaveTextContent("Pronto");

  fireEvent.click(button("Crea cartelle"));
  const dialog = await screen.findByRole("alertdialog", {
    name: "Creare 123 file in E:\\Organizzate?",
  });
  expect(dialog).toHaveTextContent("L'operazione modifica i file sul disco.");
  fireEvent.click(within(dialog).getByRole("button", { name: "Crea 123 file" }));
  await waitFor(() =>
    expect(commandsMock.startOrganize).toHaveBeenCalledWith(
      expect.objectContaining({ destination: "E:\\Organizzate", method: "hardlink" }),
    ),
  );
  expect(await screen.findByText("Creazione delle cartelle avviata")).toBeInTheDocument();
});

it("i livelli hanno pulsanti con etichetta", async () => {
  renderPage();
  expect(await screen.findByRole("button", { name: "Sposta su il livello 2" })).toHaveTextContent(
    "Sposta su",
  );
  fireEvent.click(button("Rimuovi il livello 1"));
  expect(screen.getByLabelText("Livello 1")).toHaveValue("year");
  expect(screen.queryByLabelText("Livello 2")).toBeNull();
});

it("storico leggibile; eliminare le cartelle create chiede conferma", async () => {
  commandsMock.listOrganizations.mockResolvedValue([
    { id: 12, dest: "D:\\out", method: "copy", state: "done", entries: 340 },
    { id: 11, dest: "D:\\vecchie", method: "copy", state: "undone", entries: 5 },
  ] as never);
  commandsMock.undoOrganization.mockResolvedValue([338, 2] as never);
  renderPage();
  expect(await screen.findByText("Cartelle in D:\\out")).toBeInTheDocument();
  expect(screen.getByText("Copia · 340 elementi · Completata")).toBeInTheDocument();
  expect(screen.queryByText(/#12/)).toBeNull();
  expect(screen.queryByText(/vecchie/)).toBeNull();
  fireEvent.click(button("Elimina le cartelle create…"));
  const dialog = await screen.findByRole("alertdialog", {
    name: "Eliminare le cartelle create in D:\\out?",
  });
  fireEvent.click(within(dialog).getByRole("button", { name: "Elimina 340 elementi" }));
  await waitFor(() => expect(commandsMock.undoOrganization).toHaveBeenCalledWith(12, true));
  expect(await screen.findByText("Eliminati 338 elementi, conservati 2.")).toBeInTheDocument();
});

it("la sezione Sequenza apre l'esportazione della sequenza", async () => {
  useSequence.setState({ view: "sequence" });
  renderPage();
  fireEvent.click(
    await screen.findByRole("button", { name: "Apri l'esportazione della sequenza" }),
  );
  expect(useSequence.getState().view).toBe("export");
  expect(useLayout.getState().module).toBe("sequence");
});

it("il titolo della pagina è quello della voce di navigazione", async () => {
  renderPage();
  expect(await screen.findByRole("heading", { level: 1, name: "Esporta" })).toBeInTheDocument();
});
