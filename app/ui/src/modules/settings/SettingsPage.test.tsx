import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  fireEvent.change(screen.getByLabelText("soglia", { exact: false }), {
    target: { value: "0.4" },
  });
  fireEvent.click(screen.getByLabelText("volti", { exact: false, selector: "input" }));
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
      ai: { ...config.ai, confidence_threshold: 0.4, faces_enabled: false },
      ui: { theme: "light", language: "en" },
      data_dir: "D:\\PhotoCatData",
    }),
  );
  await waitFor(() => expect(useLayout.getState().theme).toBe("light"));
  expect(useLayout.getState().language).toBe("en");
});
