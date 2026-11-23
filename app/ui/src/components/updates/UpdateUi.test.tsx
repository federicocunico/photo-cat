// Aggiornamenti dell'app (app-updates): notifica, download con avanzamento, badge e installazione.
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, emit, ipcMockModule } from "../../test/ipcMock";
import type { UpdateInfo } from "../../lib/ipc";
import { ToastHost } from "../feedback/ToastHost";
import { useToast } from "../../store/toasts";
import { useLayout } from "../../store/layout";
import {
  STARTUP_DELAY_MS,
  useStartupUpdateCheck,
  useUpdateEvents,
  useUpdates,
} from "../../store/updates";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { UpdateBadge } from "./UpdateUi";
import { UpdateSettings } from "./UpdateSettings";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

const MB = 1024 * 1024;
const PAGE = "https://github.com/federicocunico/photo-cat/releases/tag/v0.5.4";
const info = (over: Partial<UpdateInfo> = {}): UpdateInfo => ({
  version: "0.5.4",
  current: "0.5.3",
  pageUrl: PAGE,
  publishedAt: "2026-11-24T10:00:00Z",
  installer: {
    name: "photo-cat_0.5.4_x64-setup.exe",
    url: "https://github.com/federicocunico/photo-cat/releases/download/v0.5.4/photo-cat_0.5.4_x64-setup.exe",
    size: 473 * MB,
    sha256: "a".repeat(64),
  },
  downloaded: false,
  ...over,
});

function Host() {
  useUpdateEvents();
  return (
    <>
      <UpdateBadge />
      <ToastHost />
    </>
  );
}
const renderHost = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Host />
    </QueryClientProvider>,
  );
const notice = () =>
  screen.getByRole("region", { name: "È disponibile photo-cat 0.5.4 (in uso 0.5.3)" });
const check = (manual: boolean) => act(() => useUpdates.getState().check(manual));

beforeEach(() => {
  vi.clearAllMocks();
  useToast.setState({ notices: [] });
  useUpdates.setState({
    info: null,
    phase: "idle",
    done: 0,
    total: 0,
    error: null,
    dismissed: false,
    confirm: false,
    lastCheck: null,
    checking: false,
  });
  commandsMock.checkUpdate.mockResolvedValue(info());
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

describe("Controllo delle nuove versioni", () => {
  it("Versione più recente disponibile: notifica con Novità, Aggiorna e chiusura", async () => {
    renderHost();
    await check(false);
    expect(within(notice()).getByRole("button", { name: "Aggiorna" })).toBeInTheDocument();
    fireEvent.click(within(notice()).getByRole("button", { name: "Novità" }));
    expect(commandsMock.openReleasePage).toHaveBeenCalledWith(PAGE);
  });

  it("Già aggiornata: il controllo su richiesta lo dice e non mostra la notifica", async () => {
    commandsMock.checkUpdate.mockResolvedValue(null);
    renderHost();
    await check(true);
    expect(
      await screen.findByText("photo-cat 0.5.1 è la versione più recente"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: /È disponibile/ })).toBeNull();
  });

  it("Senza connessione: il controllo automatico non mostra avvisi, quello su richiesta sì", async () => {
    commandsMock.checkUpdate.mockRejectedValue({ detail: "GitHub non raggiungibile" });
    renderHost();
    await check(false);
    expect(useToast.getState().notices).toHaveLength(0);
    await check(true);
    expect(
      await screen.findByText(
        /Controllo degli aggiornamenti non riuscito: GitHub non raggiungibile/,
      ),
    ).toBeInTheDocument();
  });

  it("Notifica chiusa: non ricompare con un controllo automatico, sì con uno su richiesta", async () => {
    renderHost();
    await check(false);
    fireEvent.click(
      within(notice()).getByRole("button", { name: "Chiudi la notifica dell'aggiornamento" }),
    );
    expect(screen.queryByRole("region", { name: /È disponibile/ })).toBeNull();
    await check(false);
    expect(screen.queryByRole("region", { name: /È disponibile/ })).toBeNull();
    await check(true);
    expect(notice()).toBeInTheDocument();
  });
});

describe("Download e installazione (Windows)", () => {
  it("Download con avanzamento nella notifica e nel badge, con Interrompi", async () => {
    renderHost();
    await check(false);
    fireEvent.click(within(notice()).getByRole("button", { name: "Aggiorna" }));
    await waitFor(() => expect(commandsMock.downloadUpdate).toHaveBeenCalledWith("0.5.4"));
    act(() => emit("updateProgress", { version: "0.5.4", done: 199 * MB, total: 473 * MB }));
    const bar = within(notice()).getByRole("progressbar", { name: "Download dell'aggiornamento" });
    expect(bar).toHaveAttribute("aria-valuenow", "42");
    expect(within(notice()).getByText("42% · 199 di 473 MB")).toBeInTheDocument();
    expect(screen.getByTestId("update-badge")).toHaveTextContent("Aggiornamento 42%");
    fireEvent.click(within(notice()).getByRole("button", { name: "Interrompi" }));
    expect(commandsMock.cancelUpdateDownload).toHaveBeenCalled();
    // Interrotto: si torna alla notifica con "Aggiorna" (il prossimo download riprende).
    act(() =>
      emit("updateFailed", { version: "0.5.4", message: "operazione annullata", cancelled: true }),
    );
    expect(within(notice()).getByRole("button", { name: "Aggiorna" })).toBeInTheDocument();
  });

  it("File non integro: la notifica lo dice e offre Riprova", async () => {
    renderHost();
    await check(false);
    fireEvent.click(within(notice()).getByRole("button", { name: "Aggiorna" }));
    act(() =>
      emit("updateFailed", {
        version: "0.5.4",
        message: "SHA-256 non corrispondente",
        cancelled: false,
      }),
    );
    expect(within(notice()).getByText("SHA-256 non corrispondente")).toBeInTheDocument();
    fireEvent.click(within(notice()).getByRole("button", { name: "Riprova" }));
    await waitFor(() => expect(commandsMock.downloadUpdate).toHaveBeenCalledTimes(2));
  });

  it("Aggiorna ora: badge «Aggiornamento pronto», dialogo e avvio dell'installer; Più tardi lo lascia", async () => {
    renderHost();
    await check(false);
    fireEvent.click(within(notice()).getByRole("button", { name: "Aggiorna" }));
    act(() => emit("updateReady", { version: "0.5.4" }));
    // A download finito parla il badge.
    expect(screen.queryByRole("region", { name: /È disponibile/ })).toBeNull();
    const badge = screen.getByTestId("update-badge");
    expect(badge).toHaveTextContent("Aggiornamento pronto");

    fireEvent.click(badge);
    let dialog = await screen.findByRole("dialog", { name: "Installare photo-cat 0.5.4 ora?" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Più tardi" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByTestId("update-badge")).toHaveTextContent("Aggiornamento pronto");
    expect(commandsMock.installUpdate).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("update-badge"));
    dialog = await screen.findByRole("dialog", { name: "Installare photo-cat 0.5.4 ora?" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Aggiorna ora" }));
    await waitFor(() => expect(commandsMock.installUpdate).toHaveBeenCalledWith("0.5.4"));
  });

  it("Aggiornamento già scaricato: niente nuovo download, subito il badge", async () => {
    commandsMock.checkUpdate.mockResolvedValue(info({ downloaded: true }));
    renderHost();
    await check(false);
    expect(screen.getByTestId("update-badge")).toHaveTextContent("Aggiornamento pronto");
    expect(commandsMock.downloadUpdate).not.toHaveBeenCalled();
  });
});

describe("Aggiornamento su macOS e Linux", () => {
  it("«Aggiorna» apre la pagina della release, senza download né badge", async () => {
    commandsMock.checkUpdate.mockResolvedValue(info({ installer: null }));
    renderHost();
    await check(false);
    fireEvent.click(within(notice()).getByRole("button", { name: "Apri la pagina della release" }));
    expect(commandsMock.openReleasePage).toHaveBeenCalledWith(PAGE);
    expect(commandsMock.downloadUpdate).not.toHaveBeenCalled();
    expect(screen.queryByTestId("update-badge")).toBeNull();
  });
});

describe("Controllo all'avvio e Impostazioni › Informazioni", () => {
  afterEach(() => vi.useRealTimers());

  it("all'avvio controlla dopo un breve ritardo, se l'impostazione è attiva", async () => {
    vi.useFakeTimers();
    useLayout.setState({ checkUpdatesOnStart: true });
    renderHook(() => useStartupUpdateCheck(true));
    expect(commandsMock.checkUpdate).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(STARTUP_DELAY_MS));
    expect(commandsMock.checkUpdate).toHaveBeenCalledTimes(1);
  });

  it("Controllo automatico disattivato: nessuna richiesta all'avvio, «Controlla ora» funziona", async () => {
    vi.useFakeTimers();
    useLayout.setState({ checkUpdatesOnStart: true });
    render(<UpdateSettings />);
    fireEvent.click(
      screen.getByRole("checkbox", { name: "Controlla gli aggiornamenti all'avvio" }),
    );
    expect(useLayout.getState().checkUpdatesOnStart).toBe(false);
    renderHook(() => useStartupUpdateCheck(true));
    await act(async () => vi.advanceTimersByTime(STARTUP_DELAY_MS * 2));
    expect(commandsMock.checkUpdate).not.toHaveBeenCalled();
    vi.useRealTimers();

    fireEvent.click(screen.getByRole("button", { name: "Controlla ora" }));
    await waitFor(() => expect(commandsMock.checkUpdate).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("È disponibile photo-cat 0.5.4")).toBeInTheDocument();
  });
});
