import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import i18n from "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useToast } from "../../store/toasts";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { SettingsPage } from "./SettingsPage";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const config = {
  resources: { profile: "balanced", ram_limit_mb: null },
  ai: {
    execution_provider: "auto",
    confidence_threshold: 0.25,
    auto_download_models: false,
    faces_enabled: true,
    face_min_size_px: 40,
    ignored_folder_names: [],
    thresholds: {
      person_detection: 0.4,
      face_detection: 0.8,
      face_min_sharpness: 0.35,
      face_similarity: 0.363,
      similar_photos: 0.8,
    },
  },
  logging: { level: "info", max_files: 10, max_file_mb: 20 },
  ui: { theme: "dark", language: "it", double_click: "system" },
  data_dir: null,
  external_apps: [],
};

const taxonomy = {
  version: 3,
  groups: [
    {
      id: "environment",
      label: "Ambiente",
      threshold: 0.25,
      categories: [
        { id: "stanza", label: "Stanza", enabled: true, prompts: ["a room"] },
        { id: "teatro", label: "Teatro", enabled: true, prompts: ["a theater"] },
      ],
    },
  ],
};

const impact = {
  mediaLabelsChanged: 12,
  mediaPeopleChanged: 0,
  facesExcluded: 0,
  facesDiscardedSharpness: 0,
  personsAffectedEstimate: 0,
  reanalysisPersons: 0,
  reanalysisFaces: 0,
  similarGroupsCurrent: 0,
  similarGroups: 0,
};

const defaultThresholds = config.ai.thresholds;

beforeEach(() => {
  vi.clearAllMocks();
  useLayout.setState({ theme: "dark", language: "it", settingsSection: "general" });
  commandsMock.getConfig.mockResolvedValue(structuredClone(config) as never);
  commandsMock.getTaxonomy.mockResolvedValue(structuredClone(taxonomy) as never);
  commandsMock.systemInfo.mockResolvedValue({
    system: { totalMb: 16384, availableMb: 8000, logicalCores: 8, processMb: 300 },
    budget: { budgetMb: 4096, limitedBy: "profile" },
    providers: ["cpu"],
    unavailableProviders: [],
    onnxRuntime: "ONNX Runtime 1.24",
    executionProvider: "auto",
    resourceProfile: "balanced",
  } as never);
  commandsMock.appInfo.mockResolvedValue({
    version: "0.1.0",
    dataDir: "C:\\Users\\u\\AppData\\Local\\photo-cat",
    logsDir: "",
    previousCrash: null,
  } as never);
});
afterEach(async () => {
  checkedCleanup(A11Y_KNOWN.none)();
  await i18n.changeLanguage("it");
});

function renderSettings() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <SettingsPage />
    </QueryClientProvider>,
  );
}
const open = async (section: string) => {
  const nav = await screen.findByRole("navigation", { name: "Sezioni delle impostazioni" });
  fireEvent.click(within(nav).getByRole("button", { name: section }));
};
const applyBar = () => screen.getByRole("region", { name: "Modifiche da applicare" });

it("le sezioni sono quelle della spec e nessun Salva a metà pagina", async () => {
  renderSettings();
  const nav = await screen.findByRole("navigation", { name: "Sezioni delle impostazioni" });
  expect(
    within(nav)
      .getAllByRole("button")
      .map((b) => b.textContent),
  ).toEqual([
    "Generale",
    "Libreria e importazione",
    "Riconoscimento AI",
    "Modelli AI e sistema",
    "Applicazioni esterne",
    "Diagnostica",
    "Avanzate",
  ]);
  expect(screen.queryByRole("button", { name: /^Salva/ })).toBeNull();
});

it("le preferenze si salvano subito, con Salvato accanto al campo", async () => {
  renderSettings();
  fireEvent.click(await screen.findByRole("radio", { name: "Chiaro" }));
  expect(useLayout.getState().theme).toBe("light");
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ ui: expect.objectContaining({ theme: "light" }) }),
    ),
  );
  expect(await screen.findByText("Salvato")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("radio", { name: "English" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ ui: expect.objectContaining({ theme: "light", language: "en" }) }),
    ),
  );
  expect(useLayout.getState().language).toBe("en");
  await act(() => i18n.changeLanguage("it"));

  await open("Riconoscimento AI");
  fireEvent.click(screen.getByRole("switch", { name: "Riconoscimento volti" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ ai: expect.objectContaining({ faces_enabled: false }) }),
    ),
  );

  await open("Modelli AI e sistema");
  fireEvent.change(await screen.findByLabelText("Limite RAM (MB)"), { target: { value: "2048" } });
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ resources: { profile: "balanced", ram_limit_mb: 2048 } }),
    ),
  );
  // Nessuna preferenza ha una barra da applicare.
  expect(screen.queryByRole("region", { name: "Modifiche da applicare" })).toBeNull();
});

it("percorso dei dati: si applica con la barra, Ripristina lo annulla", async () => {
  renderSettings();
  expect(await screen.findByText(/AppData\\Local\\photo-cat/)).toBeInTheDocument();
  const path = screen.getByLabelText("Percorso dati alternativo");
  fireEvent.change(path, { target: { value: "D:\\PhotoCatData" } });
  expect(applyBar()).toHaveTextContent("Il nuovo percorso è usato dal prossimo avvio.");
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Ripristina" }));
  expect(path).toHaveValue("");
  expect(commandsMock.saveSettings).not.toHaveBeenCalled();
  fireEvent.change(path, { target: { value: "D:\\PhotoCatData" } });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ data_dir: "D:\\PhotoCatData" }),
    ),
  );
});

it("sensibilità su scala Più risultati ↔ Più precisi, applicata con la barra", async () => {
  renderSettings();
  await open("Riconoscimento AI");
  const persons = await screen.findByRole("slider", { name: "Rilevamento persone" });
  expect(persons).toHaveAttribute("aria-valuetext", "40% verso Più precisi");
  fireEvent.change(persons, { target: { value: "70" } });
  expect(persons).toHaveAttribute("aria-valuetext", "70% verso Più precisi");
  fireEvent.click(screen.getByRole("radio", { name: "Solo grandi" }));
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({
          face_min_size_px: 80,
          thresholds: { ...defaultThresholds, person_detection: 0.7 },
        }),
      }),
    ),
  );
});

it("le soglie dei gruppi in bozza restano preservate mentre si salvano le preferenze", async () => {
  renderSettings();
  await open("Riconoscimento AI");
  fireEvent.change(await screen.findByRole("slider", { name: "Ambiente" }), {
    target: { value: "50" },
  });
  fireEvent.click(screen.getByRole("switch", { name: "Riconoscimento volti" }));
  await waitFor(() => expect(commandsMock.saveSettings).toHaveBeenCalled());
  // La preferenza salvata non porta con sé le soglie da applicare.
  expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
    expect.objectContaining({
      ai: expect.objectContaining({ faces_enabled: false, thresholds: defaultThresholds }),
    }),
  );
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  const expected = structuredClone(taxonomy);
  expected.groups[0].threshold = 0.5;
  await waitFor(() => expect(commandsMock.saveTaxonomy).toHaveBeenCalledWith(expected, 3));
});

it("anteprima di una soglia di categoria e applicazione senza rianalisi", async () => {
  commandsMock.previewThresholds.mockResolvedValue(impact);
  renderSettings();
  await open("Avanzate");
  const stanza = await screen.findByLabelText("Categoria Stanza");
  expect(stanza).toHaveAttribute("placeholder", "eredita 0,25");
  fireEvent.change(stanza, { target: { value: "0.6" } });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Anteprima dell'effetto" }));
  const expected = structuredClone(taxonomy);
  (expected.groups[0].categories[0] as { threshold?: number }).threshold = 0.6;
  await waitFor(() =>
    expect(commandsMock.previewThresholds).toHaveBeenCalledWith(defaultThresholds, expected),
  );
  expect(await screen.findByText("12 foto cambiano catalogazioni")).toBeInTheDocument();
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() => expect(commandsMock.saveTaxonomy).toHaveBeenCalledWith(expected, 3));
  expect(commandsMock.saveSettings).not.toHaveBeenCalled();
});

it("Nitidezza minima dei volti: anteprima degli scarti e applicazione senza rianalisi", async () => {
  commandsMock.previewThresholds.mockResolvedValue({ ...impact, facesDiscardedSharpness: 7 });
  renderSettings();
  await open("Avanzate");
  const sharpness = await screen.findByLabelText("Nitidezza minima dei volti");
  expect(sharpness).toHaveValue("0.35");
  fireEvent.change(sharpness, { target: { value: "0.6" } });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Anteprima dell'effetto" }));
  await waitFor(() =>
    expect(commandsMock.previewThresholds).toHaveBeenCalledWith(
      { ...defaultThresholds, face_min_sharpness: 0.6 },
      null,
    ),
  );
  expect(await screen.findByText("7 volti scartati per nitidezza")).toBeInTheDocument();
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({
          thresholds: expect.objectContaining({ face_min_sharpness: 0.6 }),
        }),
      }),
    ),
  );
  expect(commandsMock.reanalyzeBelowFloor).not.toHaveBeenCalled();
});

it("rifiuta un valore non valido e ripristina i predefiniti", async () => {
  renderSettings();
  await open("Avanzate");
  await screen.findByLabelText("Categoria Stanza");
  const persons = screen.getByLabelText("Rilevamento persone");
  fireEvent.change(persons, { target: { value: "1.5" } });
  expect(screen.getByRole("alert")).toHaveTextContent("Resta in vigore 0,4");
  fireEvent.change(screen.getByLabelText("Somiglianza tra volti"), { target: { value: "0.5" } });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({
          thresholds: { ...defaultThresholds, face_similarity: 0.5 },
        }),
      }),
    ),
  );

  fireEvent.change(persons, { target: { value: "0.7" } });
  fireEvent.change(screen.getByLabelText("Nitidezza minima dei volti"), {
    target: { value: "0.8" },
  });
  fireEvent.change(screen.getByLabelText("Gruppo Ambiente"), { target: { value: "0.5" } });
  fireEvent.change(screen.getByLabelText("Categoria Teatro"), { target: { value: "0.9" } });
  fireEvent.click(screen.getByRole("button", { name: "Ripristina predefiniti" }));
  await waitFor(() => expect(persons).toHaveValue("0.4"));
  expect(screen.getByLabelText("Nitidezza minima dei volti")).toHaveValue("0.35");
  expect(screen.getByLabelText("Somiglianza tra volti")).toHaveValue("0.363");
  expect(screen.getByLabelText("Gruppo Ambiente")).toHaveValue("0.25");
  expect(screen.getByLabelText("Categoria Teatro")).toHaveValue("");
});

it("propone la rianalisi sotto il minimo salvato", async () => {
  commandsMock.previewThresholds.mockResolvedValue({ ...impact, reanalysisFaces: 30 });
  renderSettings();
  await open("Avanzate");
  fireEvent.change(await screen.findByLabelText("Rilevamento volti"), {
    target: { value: "0.7" },
  });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Anteprima dell'effetto" }));
  expect(
    await screen.findByText(/30 foto sono state analizzate con un minimo più alto/),
  ).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Rianalizza" }));
  await waitFor(() => expect(commandsMock.reanalyzeBelowFloor).toHaveBeenCalled());
  expect(commandsMock.saveSettings).toHaveBeenCalledWith(
    expect.objectContaining({
      ai: expect.objectContaining({
        thresholds: expect.objectContaining({ face_detection: 0.7 }),
      }),
    }),
  );
});

it("applicazioni esterne: eseguibile inesistente segnalato accanto, poi doppio clic", async () => {
  renderSettings();
  await open("Applicazioni esterne");
  fireEvent.click(await screen.findByRole("button", { name: "Aggiungi applicazione" }));
  // Scenario "Eseguibile inesistente": il backend rifiuta e il messaggio indica il percorso.
  commandsMock.saveSettings.mockRejectedValueOnce({
    code: "INVALID_CONFIG",
    detail: 'eseguibile di "Lightroom" non trovato: C:\\LR\\Lightroom.exe',
  });
  fireEvent.change(screen.getByLabelText("Nome"), { target: { value: "Lightroom " } });
  fireEvent.change(screen.getByLabelText("Eseguibile"), {
    target: { value: "C:\\LR\\Lightroom.exe" },
  });
  expect(await screen.findByRole("alert")).toHaveTextContent("C:\\LR\\Lightroom.exe");
  fireEvent.change(screen.getByLabelText("Argomenti (uno per riga)"), {
    target: { value: "--edit\n{file}\n" },
  });
  const app = {
    id: "app-1",
    name: "Lightroom",
    path: "C:\\LR\\Lightroom.exe",
    args: ["--edit", "{file}"],
  };
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ external_apps: [app] }),
    ),
  );
  // Scenario "Doppio clic con applicazione configurata": scelta in Generale.
  await open("Generale");
  fireEvent.change(screen.getByLabelText("Doppio clic su una miniatura"), {
    target: { value: "app-1" },
  });
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({
        external_apps: [app],
        ui: expect.objectContaining({ double_click: "app-1" }),
      }),
    ),
  );
});

it("cartelle da ignorare: propone il ricalcolo dei luoghi con un avviso", async () => {
  commandsMock.recomputePlaces.mockResolvedValue(3);
  renderSettings();
  await open("Libreria e importazione");
  fireEvent.change(
    await screen.findByLabelText("Cartelle da ignorare per il luogo (una per riga)"),
    { target: { value: "Selezionate\n\n" } },
  );
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({ ignored_folder_names: ["Selezionate"] }),
      }),
    ),
  );
  const notice = await waitFor(() => {
    const n = useToast.getState().notices.find((x) => x.key === "recompute-places");
    expect(n).toBeDefined();
    return n!;
  });
  expect(notice.action?.label).toBe("Ricalcola i luoghi");
  await act(async () => {
    await notice.action!.run();
  });
  expect(commandsMock.recomputePlaces).toHaveBeenCalled();
  expect(useToast.getState().notices.map((n) => n.message)).toContain(
    "Luogo ricalcolato per 3 foto.",
  );
});

it("gruppo nuovo solo manuale, senza avviso di rianalisi", async () => {
  renderSettings();
  await open("Riconoscimento AI");
  fireEvent.click(await screen.findByRole("button", { name: "Aggiungi gruppo" }));
  const group = screen.getByLabelText("Nome del nuovo gruppo");
  fireEvent.keyDown(group, { key: "Enter" });
  expect(await screen.findByRole("alert")).toHaveTextContent("Il nome è obbligatorio");
  fireEvent.change(group, { target: { value: "Evento" } });
  fireEvent.keyDown(group, { key: "Enter" });
  const first = await screen.findByLabelText("Nome della nuova voce in Evento");
  fireEvent.change(first, { target: { value: "Compleanno" } });
  fireEvent.keyDown(first, { key: "Enter" });
  const evento = screen.getByTestId("group-evento");
  fireEvent.click(within(evento).getByRole("button", { name: "Aggiungi voce" }));
  const second = screen.getByLabelText("Nome della nuova voce in Evento");
  fireEvent.change(second, { target: { value: "Laurea" } });
  fireEvent.keyDown(second, { key: "Enter" });
  expect(within(evento).getAllByText("manuale")).toHaveLength(2);
  expect(screen.queryByRole("note")).not.toBeInTheDocument();
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() => expect(commandsMock.saveTaxonomy).toHaveBeenCalled());
  const [saved, version] = commandsMock.saveTaxonomy.mock.calls[0] as unknown as [
    { groups: { id: string; label: string; categories: { id: string; prompts: string[] }[] }[] },
    number,
  ];
  expect(version).toBe(3);
  const added = saved.groups.find((g) => g.id === "evento")!;
  expect(added.label).toBe("Evento");
  expect(added.categories.map((c) => [c.id, c.prompts])).toEqual([
    ["compleanno", []],
    ["laurea", []],
  ]);
});

it("voce resa automatica: compare l'avviso di rianalisi", async () => {
  renderSettings();
  await open("Riconoscimento AI");
  const ambiente = await screen.findByTestId("group-environment");
  fireEvent.click(within(ambiente).getByRole("button", { name: "Aggiungi voce" }));
  const name = screen.getByLabelText("Nome della nuova voce in Ambiente");
  fireEvent.change(name, { target: { value: "Barche" } });
  fireEvent.keyDown(name, { key: "Enter" });
  expect(within(ambiente).getByText("manuale")).toBeInTheDocument();
  expect(screen.queryByRole("note")).not.toBeInTheDocument();
  fireEvent.change(
    screen.getByLabelText(
      "Descrizione per il riconoscimento automatico (inglese, facoltativa) barche",
    ),
    { target: { value: "a photo of boats" } },
  );
  expect(screen.getByRole("note")).toHaveTextContent("cambia l'analisi automatica");
  expect(within(ambiente).queryByText("manuale")).not.toBeInTheDocument();
});

it("eliminazione di una voce usata con il numero di foto", async () => {
  commandsMock.categoryUsage.mockResolvedValue(25 as never);
  commandsMock.deleteCategory.mockResolvedValue(25 as never);
  renderSettings();
  await open("Riconoscimento AI");
  fireEvent.click(await screen.findByRole("button", { name: "Elimina la voce Teatro" }));
  const ask = await screen.findByRole("alertdialog");
  expect(ask).toHaveTextContent("Eliminare «Teatro»? 25 foto perderanno questa catalogazione.");
  expect(commandsMock.categoryUsage).toHaveBeenCalledWith("environment", "teatro");
  expect(commandsMock.deleteCategory).not.toHaveBeenCalled();
  fireEvent.click(within(ask).getByRole("button", { name: "Elimina" }));
  await waitFor(() =>
    expect(commandsMock.deleteCategory).toHaveBeenCalledWith("environment", "teatro"),
  );
  expect(await screen.findByText("Voce eliminata da 25 foto")).toBeInTheDocument();
});

it("Soglia dei gruppi di foto simili: anteprima dei gruppi e ricalcolo senza modelli", async () => {
  commandsMock.previewThresholds.mockResolvedValue({
    ...impact,
    mediaLabelsChanged: 0,
    similarGroupsCurrent: 120,
    similarGroups: 85,
  });
  renderSettings();
  await open("Avanzate");
  await screen.findByLabelText("Categoria Stanza");
  const field = screen.getByLabelText("Somiglianza dei gruppi di foto simili");
  expect(field).toHaveValue("0.8");
  fireEvent.change(field, { target: { value: "1.3" } });
  expect(screen.getByRole("alert")).toHaveTextContent("Resta in vigore 0,8");
  fireEvent.change(field, { target: { value: "0.9" } });
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Anteprima dell'effetto" }));
  await waitFor(() =>
    expect(commandsMock.previewThresholds).toHaveBeenCalledWith(
      expect.objectContaining({ similar_photos: 0.9 }),
      null,
    ),
  );
  expect(await screen.findByTestId("similar-preview")).toHaveTextContent(
    "Gruppi con questa soglia: 85 (ora 120)",
  );
  fireEvent.click(within(applyBar()).getByRole("button", { name: "Applica" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({
          thresholds: expect.objectContaining({ similar_photos: 0.9 }),
        }),
      }),
    ),
  );
  expect(commandsMock.reanalyze).not.toHaveBeenCalled();
});

it("propone l'analisi dei gruppi alle librerie analizzate prima", async () => {
  commandsMock.similarPending.mockResolvedValue(1200 as never);
  renderSettings();
  await open("Riconoscimento AI");
  fireEvent.click(await screen.findByRole("button", { name: "Analizza i gruppi" }));
  await waitFor(() => expect(commandsMock.startSimilarAnalysis).toHaveBeenCalled());
});

it("ricerca: porta alla sezione ed evidenzia il campo", async () => {
  renderSettings();
  fireEvent.change(await screen.findByLabelText("Cerca nelle impostazioni"), {
    target: { value: "ram" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Limite RAM \(MB\)/ }));
  expect(useLayout.getState().settingsSection).toBe("models");
  const field = await screen.findByLabelText("Limite RAM (MB)");
  expect(field.closest("[data-setting]")).toHaveAttribute("data-highlight", "true");
  await waitFor(() => expect(field).toHaveFocus());
  fireEvent.change(screen.getByLabelText("Cerca nelle impostazioni"), {
    target: { value: "zzz" },
  });
  expect(screen.getByText("Nessuna impostazione trovata.")).toBeInTheDocument();
});

it("ricerca di 'log': Diagnostica con il campo Livello di log evidenziato", async () => {
  renderSettings();
  fireEvent.change(await screen.findByLabelText("Cerca nelle impostazioni"), {
    target: { value: "log" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Livello di log/ }));
  expect(useLayout.getState().settingsSection).toBe("diagnostics");
  const field = await screen.findByLabelText("Livello di log");
  expect(field.closest("[data-setting]")).toHaveClass("ui-highlight");
  await waitFor(() => expect(field).toHaveFocus());
});

it("Elimina tutti i dati dei volti: pericolo e conferma esplicita", async () => {
  commandsMock.faceCount.mockResolvedValue(512 as never);
  renderSettings();
  await open("Avanzate");
  const purge = await screen.findByRole("button", { name: "Elimina tutti i dati dei volti" });
  expect(purge).toHaveClass("btn-danger");
  fireEvent.click(purge);
  const ask = await screen.findByRole("alertdialog");
  await waitFor(() => expect(ask).toHaveTextContent("Eliminare definitivamente 512 volti"));
  fireEvent.click(within(ask).getByRole("button", { name: "Elimina tutti i dati dei volti" }));
  await waitFor(() => expect(commandsMock.purgeFaces).toHaveBeenCalled());
});
