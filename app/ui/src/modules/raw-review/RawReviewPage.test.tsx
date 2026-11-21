import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { open } from "@tauri-apps/plugin-dialog";
import "../../i18n";
import type { OpenedReview, RecentSession } from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useRawReview } from "../../store/rawReview";
import { RawReviewPage } from "./RawReviewPage";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const opened = (over: Partial<OpenedReview> = {}): OpenedReview => ({
  sessionId: "s1",
  session: {
    schemaVersion: 2,
    name: "Dolomiti",
    sourceRoot: "D:/Foto/Dolomiti",
    layout: null,
    variantsOutputDir: null,
    presets: ["Acros", "Velvia"],
    guidePreset: "Acros",
    panelPresets: ["Acros", "Velvia"],
    hiddenPresets: [],
    decisions: {},
    lastShotKey: null,
    createdAt: "2025-01-10T09:00:00.000Z",
    updatedAt: "2025-01-10T09:00:00.000Z",
  },
  layout: "presets",
  presets: ["Acros", "Velvia"],
  guidePreset: "Acros",
  shots: [
    { key: "dscf0001", displayName: "DSCF0001", variants: [] },
    { key: "dscf0002", displayName: "DSCF0002", variants: [] },
  ],
  warnings: [],
  ...over,
});

const recentItem = (over: Partial<RecentSession> = {}): RecentSession => ({
  sourceRoot: "D:/Foto/Roma",
  name: "Roma",
  lastUsedAt: "2025-03-02T10:00:00.000Z",
  shotCount: 40,
  choiceCount: 12,
  ...over,
});

function mount() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <RawReviewPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.rawReviewRecent.mockResolvedValue([]);
  useLayout.setState({ rawReviewTarget: null });
  useRawReview.getState().clear();
});
afterEach(cleanup);

describe("Varianti RAW", () => {
  it("mostra il titolo, lo stato vuoto e le azioni di apertura", async () => {
    mount();
    expect(
      screen.getByRole("heading", { level: 1, name: "Varianti RAW (Fujifilm X-T5)" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Nessuna cartella aperta" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apri cartella…" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Apri progetto FujiView (.fujiview)…" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("Nessuna sessione recente.")).toBeInTheDocument();
  });

  it("apre la cartella scelta e mostra il riepilogo con gli avvisi", async () => {
    vi.mocked(open).mockResolvedValue("D:/Foto/Dolomiti");
    commandsMock.rawReviewOpen.mockResolvedValue(opened({ warnings: ["Duplicato ignorato"] }));
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Apri cartella…" }));
    expect(await screen.findByRole("heading", { name: "Dolomiti" })).toBeInTheDocument();
    expect(commandsMock.rawReviewOpen).toHaveBeenCalledWith("D:/Foto/Dolomiti");
    expect(vi.mocked(open).mock.calls[0][0]).toMatchObject({ directory: true });
    expect(screen.getByText("2 scatti")).toBeInTheDocument();
    expect(screen.getByText("Una sottocartella per preset")).toBeInTheDocument();
    expect(screen.getByText("Duplicato ignorato")).toBeInTheDocument();
  });

  it("non apre nulla se il dialogo viene annullato", async () => {
    vi.mocked(open).mockResolvedValue(null);
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Apri cartella…" }));
    await act(async () => {});
    expect(commandsMock.rawReviewOpen).not.toHaveBeenCalled();
  });

  it("spiega le forme supportate per una cartella senza foto", async () => {
    vi.mocked(open).mockResolvedValue("D:/Vuota");
    commandsMock.rawReviewOpen.mockRejectedValue({
      code: "INVALID_INPUT",
      detail: "La cartella non contiene foto",
    });
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Apri cartella…" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Cartella non supportata");
    expect(alert).toHaveTextContent("file RAF al primo livello");
    expect(alert).toHaveTextContent("una sottocartella per preset");
    // Si resta nello stato iniziale.
    expect(screen.getByRole("heading", { name: "Nessuna cartella aperta" })).toBeInTheDocument();
  });

  it("apre un progetto FujiView con il filtro .fujiview", async () => {
    vi.mocked(open).mockResolvedValue("D:/Progetti/dolomiti.fujiview");
    commandsMock.rawReviewOpenFujiview.mockResolvedValue(opened());
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Apri progetto FujiView (.fujiview)…" }));
    expect(await screen.findByRole("heading", { name: "Dolomiti" })).toBeInTheDocument();
    expect(commandsMock.rawReviewOpenFujiview).toHaveBeenCalledWith(
      "D:/Progetti/dolomiti.fujiview",
    );
    expect(vi.mocked(open).mock.calls[0][0]).toMatchObject({
      directory: false,
      filters: [{ name: "FujiView", extensions: ["fujiview"] }],
    });
  });

  it("mostra l'errore di un progetto non valido con il suo dettaglio", async () => {
    vi.mocked(open).mockResolvedValue("D:/rotto.fujiview");
    commandsMock.rawReviewOpenFujiview.mockRejectedValue({
      code: "INVALID_INPUT",
      detail: "JSON non valido",
    });
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Apri progetto FujiView (.fujiview)…" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Impossibile aprire la cartella");
    expect(alert).toHaveTextContent("JSON non valido");
  });

  it("elenca le sessioni recenti e ne apre una", async () => {
    commandsMock.rawReviewRecent.mockResolvedValue([recentItem()]);
    commandsMock.rawReviewOpen.mockResolvedValue(opened());
    mount();
    const row = await screen.findByRole("button", { name: /^Roma/ });
    expect(row).toHaveTextContent("D:/Foto/Roma");
    expect(row).toHaveTextContent("40 scatti · 12 scelte");
    fireEvent.click(row);
    expect(await screen.findByRole("heading", { name: "Dolomiti" })).toBeInTheDocument();
    expect(commandsMock.rawReviewOpen).toHaveBeenCalledWith("D:/Foto/Roma");
  });

  it("rimuove una sessione dall'elenco senza aprirla", async () => {
    commandsMock.rawReviewRecent
      .mockResolvedValueOnce([recentItem()])
      .mockResolvedValue([] as RecentSession[]);
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "Rimuovi Roma dall'elenco" }));
    await waitFor(() => expect(commandsMock.rawReviewForget).toHaveBeenCalledWith("D:/Foto/Roma"));
    expect(await screen.findByText("Nessuna sessione recente.")).toBeInTheDocument();
    expect(commandsMock.rawReviewOpen).not.toHaveBeenCalled();
  });

  it("apre la cartella indicata dalla destinazione e la consuma", async () => {
    commandsMock.rawReviewOpen.mockResolvedValue(opened());
    useLayout.setState({ rawReviewTarget: { root: "D:/Foto/Dolomiti" } });
    mount();
    expect(await screen.findByRole("heading", { name: "Dolomiti" })).toBeInTheDocument();
    expect(commandsMock.rawReviewOpen).toHaveBeenCalledTimes(1);
    expect(commandsMock.rawReviewOpen).toHaveBeenCalledWith("D:/Foto/Dolomiti");
    expect(useLayout.getState().rawReviewTarget).toBeNull();
  });

  it("chiude la cartella salvando la sessione e torna all'elenco", async () => {
    commandsMock.rawReviewOpen.mockResolvedValue(opened());
    useLayout.setState({ rawReviewTarget: { root: "D:/Foto/Dolomiti" } });
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "Chiudi la cartella" }));
    expect(
      await screen.findByRole("heading", { name: "Nessuna cartella aperta" }),
    ).toBeInTheDocument();
    expect(commandsMock.rawReviewSave).toHaveBeenCalledWith("s1");
    expect(commandsMock.rawReviewClose).toHaveBeenCalledWith("s1");
  });
});
