import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import i18n from "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
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
  },
  logging: { level: "info", max_files: 10, max_file_mb: 20 },
  ui: { theme: "dark", language: "it" },
  data_dir: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  useLayout.setState({ theme: "dark", language: "it" });
  commandsMock.getConfig.mockResolvedValue(structuredClone(config) as never);
  commandsMock.systemInfo.mockResolvedValue({ providers: ["cpu", "directml"] } as never);
  commandsMock.appInfo.mockResolvedValue({
    version: "0.1.0",
    dataDir: "C:\\Users\\u\\AppData\\Local\\photo-cat",
    logsDir: "",
    previousCrash: null,
  } as never);
  commandsMock.getTaxonomy.mockResolvedValue({ version: 1, groups: [] } as never);
});
afterEach(async () => {
  cleanup();
  await i18n.changeLanguage("it");
});

it("salva profilo, limite RAM, soglia, volti, tema, lingua e percorso dati", async () => {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <SettingsPage />
    </QueryClientProvider>,
  );
  const profile = await screen.findByLabelText("Profilo risorse", { exact: false });
  expect(profile).toHaveValue("balanced");
  fireEvent.change(profile, { target: { value: "light" } });
  fireEvent.change(screen.getByLabelText("RAM", { exact: false }), {
    target: { value: "2048" },
  });
  fireEvent.change(screen.getByLabelText("Soglia di confidenza", { exact: false }), {
    target: { value: "0.4" },
  });
  fireEvent.click(
    screen.getByLabelText("Riconoscimento volti", { exact: false, selector: "input" }),
  );
  fireEvent.change(screen.getByLabelText("Tema", { exact: false }), {
    target: { value: "light" },
  });
  fireEvent.change(screen.getByLabelText("Lingua", { exact: false }), {
    target: { value: "en" },
  });
  expect(screen.getByText(/AppData\\Local\\photo-cat/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Percorso dati", { exact: false }), {
    target: { value: "D:\\PhotoCatData" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));

  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith({
      ...config,
      resources: { profile: "light", ram_limit_mb: 2048 },
      ai: {
        ...config.ai,
        confidence_threshold: 0.4,
        faces_enabled: false,
        ignored_folder_names: [],
      },
      ui: { theme: "light", language: "en" },
      data_dir: "D:\\PhotoCatData",
      external_apps: [],
    }),
  );
  await waitFor(() => expect(useLayout.getState().theme).toBe("light"));
  expect(useLayout.getState().language).toBe("en");
});

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

const fullConfig = {
  ...config,
  ai: {
    ...config.ai,
    ignored_folder_names: [],
    thresholds: { person_detection: 0.4, face_detection: 0.8, face_similarity: 0.363 },
  },
  ui: { ...config.ui, double_click: "system" },
  external_apps: [],
};

function renderSettings() {
  commandsMock.getConfig.mockResolvedValue(structuredClone(fullConfig) as never);
  commandsMock.getTaxonomy.mockResolvedValue(structuredClone(taxonomy) as never);
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <SettingsPage />
    </QueryClientProvider>,
  );
}

const impact = {
  mediaLabelsChanged: 12,
  mediaPeopleChanged: 0,
  facesExcluded: 0,
  personsAffectedEstimate: 0,
  reanalysisPersons: 0,
  reanalysisFaces: 0,
};

it("anteprima di una soglia di categoria e applicazione senza rianalisi", async () => {
  commandsMock.previewThresholds.mockResolvedValue(impact);
  renderSettings();
  const stanza = await screen.findByLabelText("Categoria Stanza");
  expect(stanza).toHaveAttribute("placeholder", "eredita 0,25");
  fireEvent.change(stanza, { target: { value: "0.6" } });
  fireEvent.click(screen.getByRole("button", { name: "Anteprima dell'effetto" }));
  const expected = structuredClone(taxonomy);
  (expected.groups[0].categories[0] as { threshold?: number }).threshold = 0.6;
  await waitFor(() =>
    expect(commandsMock.previewThresholds).toHaveBeenCalledWith(
      { person_detection: 0.4, face_detection: 0.8, face_similarity: 0.363 },
      expected,
    ),
  );
  expect(await screen.findByText("12 foto cambiano catalogazioni")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Applica soglie" }));
  await waitFor(() => expect(commandsMock.saveTaxonomy).toHaveBeenCalledWith(expected, 3));
  expect(commandsMock.saveSettings).not.toHaveBeenCalled();
});

it("rifiuta un valore non valido e ripristina i predefiniti", async () => {
  renderSettings();
  const persons = await screen.findByLabelText("Rilevamento persone");
  // Scenario "Valore non valido".
  fireEvent.change(persons, { target: { value: "1.5" } });
  expect(screen.getByRole("alert")).toHaveTextContent("Resta in vigore 0,4");
  fireEvent.change(screen.getByLabelText("Somiglianza tra volti"), { target: { value: "0.5" } });
  fireEvent.click(screen.getByRole("button", { name: "Applica soglie" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        ai: expect.objectContaining({
          thresholds: { person_detection: 0.4, face_detection: 0.8, face_similarity: 0.5 },
        }),
      }),
    ),
  );

  // Scenario "Ripristino dei predefiniti".
  fireEvent.change(persons, { target: { value: "0.7" } });
  fireEvent.change(screen.getByLabelText("Gruppo Ambiente"), { target: { value: "0.5" } });
  fireEvent.change(screen.getByLabelText("Categoria Teatro"), { target: { value: "0.9" } });
  fireEvent.click(screen.getByRole("button", { name: "Ripristina predefiniti" }));
  await waitFor(() => expect(persons).toHaveValue("0.4"));
  expect(screen.getByLabelText("Somiglianza tra volti")).toHaveValue("0.363");
  expect(screen.getByLabelText("Gruppo Ambiente")).toHaveValue("0.25");
  expect(screen.getByLabelText("Categoria Teatro")).toHaveValue("");
});

it("propone la rianalisi sotto il minimo salvato", async () => {
  commandsMock.previewThresholds.mockResolvedValue({ ...impact, reanalysisFaces: 30 });
  renderSettings();
  fireEvent.change(await screen.findByLabelText("Rilevamento volti"), {
    target: { value: "0.7" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Anteprima dell'effetto" }));
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

it("applicazioni esterne: aggiunta, doppio clic ed eseguibile inesistente", async () => {
  renderSettings();
  fireEvent.click(await screen.findByRole("button", { name: "Aggiungi applicazione" }));
  fireEvent.change(screen.getByLabelText("Nome"), { target: { value: "Lightroom " } });
  fireEvent.change(screen.getByLabelText("Eseguibile"), {
    target: { value: "C:\\LR\\Lightroom.exe" },
  });
  fireEvent.change(screen.getByLabelText("Argomenti (uno per riga)"), {
    target: { value: "--edit\n{file}\n" },
  });
  // Scenario "Doppio clic con applicazione configurata": scelta nelle impostazioni.
  fireEvent.change(screen.getByLabelText("Doppio clic su una miniatura"), {
    target: { value: "app-1" },
  });
  // Scenario "Eseguibile inesistente": il backend rifiuta e il messaggio indica il percorso.
  commandsMock.saveSettings.mockRejectedValueOnce({
    code: "INVALID_CONFIG",
    detail: 'eseguibile di "Lightroom" non trovato: C:\\LR\\Lightroom.exe',
  });
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("C:\\LR\\Lightroom.exe");
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  await waitFor(() =>
    expect(commandsMock.saveSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({
        external_apps: [
          {
            id: "app-1",
            name: "Lightroom",
            path: "C:\\LR\\Lightroom.exe",
            args: ["--edit", "{file}"],
          },
        ],
        ui: expect.objectContaining({ double_click: "app-1" }),
      }),
    ),
  );
});

it("cartelle da ignorare: propone il ricalcolo dei luoghi", async () => {
  commandsMock.recomputePlaces.mockResolvedValue(3);
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
  renderSettings();
  fireEvent.change(
    await screen.findByLabelText("Cartelle da ignorare per il luogo (una per riga)"),
    {
      target: { value: "Selezionate\n\n" },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "Salva" }));
  await waitFor(() => expect(commandsMock.recomputePlaces).toHaveBeenCalled());
  expect(commandsMock.saveSettings).toHaveBeenCalledWith(
    expect.objectContaining({
      ai: expect.objectContaining({ ignored_folder_names: ["Selezionate"] }),
    }),
  );
  expect(await screen.findByText("Luogo ricalcolato per 3 foto.")).toBeInTheDocument();
  confirm.mockRestore();
});

it("gruppo nuovo solo manuale, senza avviso di rianalisi", async () => {
  renderSettings();
  fireEvent.click(await screen.findByRole("button", { name: "Aggiungi gruppo" }));
  const group = screen.getByLabelText("Nome del nuovo gruppo");
  fireEvent.keyDown(group, { key: "Enter" });
  expect(await screen.findByRole("alert")).toHaveTextContent("Il nome è obbligatorio");
  fireEvent.change(group, { target: { value: "Evento" } });
  fireEvent.keyDown(group, { key: "Enter" });
  // Il gruppo nuovo apre subito il campo per la prima voce.
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
  fireEvent.click(screen.getByRole("button", { name: "Salva tassonomia" }));
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
