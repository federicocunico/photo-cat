import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import type { ExportPreflight, SequenceExportOptions } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useSequence } from "../../store/sequence";
import { SequenceExport } from "./SequenceExport";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => "E:\\Export\\USA") }));

/** Anteprima come la calcolerebbe il backend per tre foto. */
function names(template: string) {
  const rows = [
    ["DSCF4412.png", "0001", "1043", "104300", "Paesaggi"],
    ["IMG_8812.HEIC", "0002", "1044", "104412", "Paesaggi"],
    ["DSCF4414.png", "0003", "1052", "105200", ""],
  ];
  if (template.includes("{camera}"))
    return {
      names: [],
      errors: [{ kind: "unknown_token" as const, start: 0, end: 8, text: "{camera}" }],
    };
  return {
    errors: [],
    names: rows.map(([orig, seq, time, full, tag]) => ({
      original: orig,
      name:
        template
          .replace("{seq:04d}", seq)
          .replace("{date}", "2026-08-19")
          .replace("{time_full}", full)
          .replace("{time}", time)
          .replace(tag ? "{tag}" : "_{tag}", tag) + ".jpg",
    })),
  };
}

const ok = (o: SequenceExportOptions): ExportPreflight => ({
  count: 912,
  untagged: 7,
  requiredBytes: 30.8e9,
  freeBytes: 212e9,
  hardlinks: true,
  existing: [],
  templateErrors: [],
  preview: [],
  canStart: !o.template.includes("{camera}"),
  blocked: null,
});

function renderView() {
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <SequenceExport />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useSequence.setState({ scope: { scope: "catalog", id: 3 }, view: "export" });
  commandsMock.sequenceExportPreview.mockImplementation(async (o) => names(o.template));
  commandsMock.sequenceExportPreflight.mockImplementation(async (o) => ok(o));
});
afterEach(cleanup);

it("updates the preview while the template is edited", async () => {
  renderView();
  expect(await screen.findByText("0001_2026-08-19_1043_Paesaggi.jpg")).toBeInTheDocument();
  expect(screen.getByText("0003_2026-08-19_1052.jpg")).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Modello"), {
    target: { value: "{seq:04d}_{date}_{time_full}_{tag}" },
  });
  expect(await screen.findByText("0001_2026-08-19_104300_Paesaggi.jpg")).toBeInTheDocument();
  expect(commandsMock.sequenceExportPreview).toHaveBeenLastCalledWith(
    expect.objectContaining({
      template: "{seq:04d}_{date}_{time_full}_{tag}",
      scope: { scope: "catalog", id: 3 },
    }),
    5,
  );
});

it("inserts a token chip at the cursor", async () => {
  renderView();
  const input = (await screen.findByLabelText("Modello")) as HTMLInputElement;
  fireEvent.change(input, { target: { value: "{seq:04d}_" } });
  input.setSelectionRange(10, 10);
  fireEvent.click(screen.getByRole("button", { name: /^Inserisci \{place\}/ }));
  expect(input).toHaveValue("{seq:04d}_{place}");
});

it("an unknown token is highlighted and disables the start", async () => {
  renderView();
  fireEvent.click(await screen.findByRole("button", { name: "Sfoglia" }));
  const start = await screen.findByRole("button", { name: "Avvia export (912 foto)" });
  await waitFor(() => expect(start).toBeEnabled());
  fireEvent.change(screen.getByLabelText("Modello"), { target: { value: "{camera}_{seq}" } });
  expect(await screen.findByText("Token sconosciuto: {camera}")).toBeInTheDocument();
  expect(screen.getByLabelText("Modello")).toHaveAttribute("aria-invalid", "true");
  await waitFor(() => expect(start).toBeDisabled());
});

it("insufficient space blocks the start and shows both values", async () => {
  commandsMock.sequenceExportPreflight.mockImplementation(async (o) => ({
    ...ok(o),
    requiredBytes: 31e9,
    freeBytes: 20e9,
    canStart: false,
    blocked: "Spazio insufficiente: servono 31.0 GB e la destinazione ne ha 20.0 GB.",
  }));
  renderView();
  fireEvent.change(await screen.findByLabelText("Destinazione"), {
    target: { value: "E:\\Export" },
  });
  const alert = await screen.findByText(/Spazio insufficiente/);
  expect(alert).toHaveTextContent("31.0 GB");
  expect(alert).toHaveTextContent("20.0 GB");
  expect(screen.getByRole("button", { name: /Avvia export/ })).toBeDisabled();
  expect(commandsMock.startSequenceExport).not.toHaveBeenCalled();
});

it("starts the export as a job with the chosen options", async () => {
  renderView();
  fireEvent.click(await screen.findByRole("button", { name: "Sfoglia" }));
  expect(await screen.findByText("Verifica preliminare superata")).toBeInTheDocument();
  expect(screen.getByText(/7 foto senza tag nel set/)).toBeInTheDocument();
  const start = screen.getByRole("button", { name: "Avvia export (912 foto)" });
  await waitFor(() => expect(start).toBeEnabled());
  fireEvent.click(start);
  await waitFor(() =>
    expect(commandsMock.startSequenceExport).toHaveBeenCalledWith(
      expect.objectContaining({
        destination: "E:\\Export\\USA",
        perTag: true,
        zip: true,
        zipPerTag: true,
        tagSet: 2,
        jpeg: expect.objectContaining({ quality: 97, chroma444: true, verify: true }),
      }),
    ),
  );
  expect(await screen.findByText("Export avviato.")).toBeInTheDocument();
});
