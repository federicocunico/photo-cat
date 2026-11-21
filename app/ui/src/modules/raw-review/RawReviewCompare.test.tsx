import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import "../../i18n";
import type {
  ImageMetadata,
  OpenedReview,
  PanelState,
  ReviewVariantView,
  ShotDecision,
} from "../../bindings";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { useRawReview } from "../../store/rawReview";
import { useToast } from "../../store/toasts";
import { RawReviewPage } from "./RawReviewPage";
import { forgetMetadata } from "./useShotMetadata";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (key: string, scheme: string) => `${scheme}://${key}`,
}));

const variant = (preset: string, shot: string, over: Partial<ReviewVariantView> = {}) => ({
  preset,
  fileName: `${shot}-${preset}.jpg`,
  extension: "jpg",
  bytes: 2_000_000,
  kind: "jpeg" as ReviewVariantView["kind"],
  selectable: true,
  exportable: true,
  ...over,
});

const review = (over: Partial<OpenedReview> = {}): OpenedReview => ({
  sessionId: "s1",
  session: {
    schemaVersion: 2,
    name: "Dolomiti",
    sourceRoot: "D:/Foto/Dolomiti",
    layout: null,
    variantsOutputDir: null,
    presets: ["Acros", "Velvia", "Provia"],
    guidePreset: "Acros",
    panelPresets: ["Acros", "Velvia", "Provia"],
    hiddenPresets: [],
    decisions: {
      dscf0001: { key: "dscf0001", rating: 0, winnerPreset: null, reviewed: false },
      dscf0002: { key: "dscf0002", rating: 3, winnerPreset: "Velvia", reviewed: true },
    },
    lastShotKey: null,
    createdAt: "2025-01-10T09:00:00.000Z",
    updatedAt: "2025-01-10T09:00:00.000Z",
  },
  layout: "presets",
  presets: ["Acros", "Velvia", "Provia"],
  guidePreset: "Acros",
  shots: ["dscf0001", "dscf0002"].map((key) => ({
    key,
    displayName: key.toUpperCase(),
    variants: ["Acros", "Velvia", "Provia"].map((p) => variant(p, key)),
  })),
  warnings: [],
  ...over,
});

const metadata: ImageMetadata = {
  sections: [
    {
      title: "shot",
      rows: [
        { label: "exposureTime", value: "1/250 s", valueKey: null },
        { label: "meteringMode", value: "Multi-zona", valueKey: "meteringMultiSegment" },
      ],
    },
    { title: "place", rows: [{ label: "coordinates", value: "46.5, 11.8", valueKey: null }] },
  ],
  gps: { latitude: 46.5, longitude: 11.8, altitude: null },
  recipe: {
    summary: "Velvia · DR200 · Auto",
    film: "Velvia",
    filmKey: null,
    dynamicRange: "DR200",
    whiteBalance: "Auto",
    whiteBalanceKey: "wbAuto",
    rows: [],
  },
};

function mount() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <RawReviewPage />
    </QueryClientProvider>,
  );
}

const panelNames = () =>
  screen
    .getAllByRole("region")
    .filter((el) => el.classList.contains("rr-panel"))
    .map((el) => el.getAttribute("aria-label"));

beforeEach(() => {
  vi.clearAllMocks();
  useLayout.setState({ rawReviewTarget: null });
  useRawReview.getState().clear();
  useToast.setState({ notices: [] });
  for (const id of ["s1", "s2"]) forgetMetadata(id);
  commandsMock.rawReviewRecent.mockResolvedValue([]);
  commandsMock.rawReviewImageKeys.mockImplementation(async (_s: string, shot: string) =>
    ["Acros", "Velvia", "Provia"].map((preset) => ({ preset, key: `${shot}-${preset}` })),
  );
  commandsMock.rawReviewMetadata.mockResolvedValue(metadata);
});
afterEach(cleanup);

async function openReview(opened = review()) {
  commandsMock.rawReviewOpen.mockResolvedValue(opened);
  useLayout.setState({ rawReviewTarget: { root: opened.session.sourceRoot } });
  const view = mount();
  await screen.findByRole("heading", { name: opened.session.name });
  return view;
}

describe("confronto nel modulo", () => {
  it("mostra un pannello per preset nell'ordine salvato, con barra e avvisi", async () => {
    await openReview(review({ warnings: ["Duplicato ignorato"] }));
    expect(panelNames()).toEqual(["Acros", "Velvia", "Provia"]);
    expect(screen.getByText("Duplicato ignorato")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Chiudi la cartella" })).toBeInTheDocument();
    expect(screen.getByText("1 / 2 · DSCF0001")).toBeInTheDocument();
  });

  it("riparte dall'ultimo scatto visto e ricorda quello a cui si passa", async () => {
    const opened = review();
    opened.session.lastShotKey = "dscf0002";
    await openReview(opened);
    expect(screen.getByText("2 / 2 · DSCF0002")).toBeInTheDocument();
    // Il Rating del secondo scatto mostra la sua scelta: Velvia è evidenziata.
    expect(screen.getByRole("region", { name: "Velvia" })).toHaveClass("is-winner");
    fireEvent.click(screen.getByRole("button", { name: "Scatto precedente" }));
    expect(commandsMock.rawReviewSetLastShot).toHaveBeenCalledWith("s1", "dscf0001");
    expect(screen.getByText("1 / 2 · DSCF0001")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Velvia" })).not.toHaveClass("is-winner");
    expect(screen.getByRole("button", { name: "Scatto precedente" })).toBeDisabled();
  });

  it("sceglie la variante migliore e la segna come rivista", async () => {
    await openReview();
    commandsMock.rawReviewSetWinner.mockResolvedValue({
      key: "dscf0001",
      rating: 0,
      winnerPreset: "Provia",
      reviewed: false,
    } satisfies ShotDecision);
    commandsMock.rawReviewSetReviewed.mockResolvedValue({
      key: "dscf0001",
      rating: 0,
      winnerPreset: "Provia",
      reviewed: true,
    });
    const provia = screen.getByRole("region", { name: "Provia" });
    fireEvent.click(within(provia).getByRole("button", { name: /Scegli «Provia»/ }));
    await waitFor(() => expect(provia).toHaveClass("is-winner"));
    expect(commandsMock.rawReviewSetWinner).toHaveBeenCalledWith("s1", "dscf0001", "Provia");
    expect(commandsMock.rawReviewSetReviewed).toHaveBeenCalledWith("s1", "dscf0001", true);
    expect(screen.getByText("2 scelte")).toBeInTheDocument();
  });

  it("nasconde un preset dal pannello e lo ritrova nell'elenco", async () => {
    await openReview();
    commandsMock.rawReviewTogglePreset.mockResolvedValue({
      panelPresets: ["Acros", "Provia"],
      hiddenPresets: ["Velvia"],
    } satisfies PanelState);
    fireEvent.click(screen.getByRole("button", { name: "Nascondi il pannello «Velvia»" }));
    await waitFor(() => expect(panelNames()).toEqual(["Acros", "Provia"]));
    expect(commandsMock.rawReviewTogglePreset).toHaveBeenCalledWith("s1", "Velvia");
    expect(screen.getByText("2 di 3 visibili")).toBeInTheDocument();
    const checkbox = screen.getByRole("checkbox", { name: /^Velvia/ });
    expect(checkbox).not.toBeChecked();

    commandsMock.rawReviewTogglePreset.mockResolvedValue({
      panelPresets: ["Acros", "Provia", "Velvia"],
      hiddenPresets: [],
    });
    fireEvent.click(checkbox);
    await waitFor(() => expect(panelNames()).toEqual(["Acros", "Provia", "Velvia"]));
  });

  it("il pannello non si nasconde se il backend rifiuta e l'errore si segnala", async () => {
    await openReview();
    commandsMock.rawReviewTogglePreset.mockRejectedValue({ code: "INVALID_INPUT", detail: "no" });
    fireEvent.click(screen.getByRole("button", { name: "Nascondi il pannello «Velvia»" }));
    await waitFor(() => expect(useToast.getState().notices).toHaveLength(1));
    expect(panelNames()).toEqual(["Acros", "Velvia", "Provia"]);
  });

  it("riordina i pannelli con i pulsanti dell'elenco e l'ordine resta", async () => {
    await openReview();
    commandsMock.rawReviewMovePreset.mockResolvedValue({
      panelPresets: ["Velvia", "Acros", "Provia"],
      hiddenPresets: [],
    });
    fireEvent.click(screen.getByRole("button", { name: "Sposta «Velvia» prima" }));
    await waitFor(() => expect(panelNames()).toEqual(["Velvia", "Acros", "Provia"]));
    expect(commandsMock.rawReviewMovePreset).toHaveBeenCalledWith("s1", "Velvia", 0);
    expect(useRawReview.getState().opened?.session.panelPresets).toEqual([
      "Velvia",
      "Acros",
      "Provia",
    ]);
    // Il primo non si sposta più in su, l'ultimo non scende.
    expect(screen.getByRole("button", { name: "Sposta «Velvia» prima" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Sposta «Provia» dopo" })).toBeDisabled();
  });

  it("ripristina l'ordine se il backend rifiuta lo spostamento", async () => {
    await openReview();
    commandsMock.rawReviewMovePreset.mockRejectedValue({ code: "INVALID_INPUT", detail: "no" });
    fireEvent.click(screen.getByRole("button", { name: "Sposta «Velvia» prima" }));
    await waitFor(() => expect(useToast.getState().notices).toHaveLength(1));
    expect(panelNames()).toEqual(["Acros", "Velvia", "Provia"]);
  });

  it("l'ultimo pannello visibile non si può nascondere dall'elenco", async () => {
    const opened = review();
    opened.session.panelPresets = ["Acros"];
    opened.session.hiddenPresets = ["Velvia", "Provia"];
    await openReview(opened);
    expect(screen.getByRole("checkbox", { name: /^Acros/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Mostra tutti" })).toBeInTheDocument();
    commandsMock.rawReviewShowAllPresets.mockResolvedValue({
      panelPresets: ["Acros", "Velvia", "Provia"],
      hiddenPresets: [],
    });
    fireEvent.click(screen.getByRole("button", { name: "Mostra tutti" }));
    await waitFor(() => expect(panelNames()).toEqual(["Acros", "Velvia", "Provia"]));
    expect(screen.queryByRole("button", { name: "Mostra tutti" })).toBeNull();
  });

  it("l'Anteprima RAW è un pannello come gli altri ma non si sceglie", async () => {
    const opened = review({
      layout: "rafOnly",
      presets: [],
      guidePreset: null,
      shots: [
        {
          key: "dscf0001",
          displayName: "DSCF0001",
          variants: [
            variant("raw-preview", "dscf0001", {
              extension: "jpg",
              selectable: false,
              exportable: false,
            }),
          ],
        },
      ],
    });
    opened.session.panelPresets = ["raw-preview"];
    opened.session.decisions = {
      dscf0001: { key: "dscf0001", rating: 0, winnerPreset: null, reviewed: false },
    };
    commandsMock.rawReviewImageKeys.mockResolvedValue([
      { preset: "raw-preview", key: "dscf0001-raw" },
    ]);
    await openReview(opened);
    const panel = screen.getByRole("region", { name: "Anteprima RAW" });
    // Il pulsante della scelta è disattivato con il motivo; Invio sul pannello lo spiega.
    const choose = within(panel).getByRole("button", { name: /Scegli «Anteprima RAW»/ });
    expect(choose).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(choose);
    expect(useToast.getState().notices).toHaveLength(0);
    fireEvent.keyDown(window, { key: "Enter" });
    expect(commandsMock.rawReviewSetWinner).not.toHaveBeenCalled();
    expect(useToast.getState().notices[0].message).toContain("sviluppa lo scatto");
    expect(panel).not.toHaveClass("is-winner");
  });

  it("precarica gli scatti vicini con i preset visibili", async () => {
    await openReview();
    await waitFor(() =>
      expect(commandsMock.rawReviewPrefetch).toHaveBeenCalledWith("s1", "dscf0001", [
        "Acros",
        "Velvia",
        "Provia",
      ]),
    );
  });
});

describe("barra delle informazioni", () => {
  it("traduce sezioni, etichette e valori dei metadati e copia le coordinate", async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await openReview();
    expect(await screen.findByRole("heading", { name: "Scatto" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Luogo" })).toBeInTheDocument();
    expect(screen.getByText("Tempo")).toBeInTheDocument();
    expect(screen.getByText("Esposimetro")).toBeInTheDocument();
    expect(screen.getByText("Multi-zona")).toBeInTheDocument();
    expect(commandsMock.rawReviewMetadata).toHaveBeenCalledWith("s1", "dscf0001", "Velvia");
    fireEvent.click(screen.getByRole("button", { name: "Copia coordinate" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("46.500000, 11.800000"));
    expect(await screen.findByRole("button", { name: "Copiate" })).toBeInTheDocument();
  });

  it("mostra il riepilogo della ricetta nell'intestazione dei pannelli", async () => {
    await openReview();
    const panel = screen.getByRole("region", { name: "Velvia" });
    expect(await within(panel).findByText("Velvia · DR200 · Auto")).toBeInTheDocument();
  });
});

describe("sessione tra un modulo e l'altro", () => {
  it("ritrova la sessione dopo lo smontaggio del modulo, salvandola", async () => {
    const view = await openReview();
    commandsMock.rawReviewMovePreset.mockResolvedValue({
      panelPresets: ["Velvia", "Acros", "Provia"],
      hiddenPresets: [],
    });
    fireEvent.click(screen.getByRole("button", { name: "Sposta «Velvia» prima" }));
    await waitFor(() => expect(panelNames()[0]).toBe("Velvia"));

    view.unmount();
    expect(commandsMock.rawReviewSave).toHaveBeenCalledWith("s1");
    // Chiudere il modulo non chiude la sessione nel backend.
    expect(commandsMock.rawReviewClose).not.toHaveBeenCalled();

    commandsMock.rawReviewOpen.mockClear();
    mount();
    expect(screen.getByRole("heading", { name: "Dolomiti" })).toBeInTheDocument();
    expect(panelNames()).toEqual(["Velvia", "Acros", "Provia"]);
    expect(commandsMock.rawReviewOpen).not.toHaveBeenCalled();
  });

  it("riapre la scelta della cartella se nessuna sessione era aperta", async () => {
    mount();
    expect(screen.getByRole("heading", { name: "Nessuna cartella aperta" })).toBeInTheDocument();
    await act(async () => {});
  });

  it("chiudere la cartella dimentica la sessione", async () => {
    const view = await openReview();
    fireEvent.click(screen.getByRole("button", { name: "Chiudi la cartella" }));
    await screen.findByRole("heading", { name: "Nessuna cartella aperta" });
    expect(useRawReview.getState().opened).toBeNull();
    view.unmount();
    mount();
    expect(screen.getByRole("heading", { name: "Nessuna cartella aperta" })).toBeInTheDocument();
  });

  it("aprendo un'altra cartella salva e chiude la sessione precedente", async () => {
    await openReview();
    const other = review({ sessionId: "s2" });
    other.session.name = "Roma";
    other.session.sourceRoot = "D:/Foto/Roma";
    commandsMock.rawReviewOpen.mockResolvedValue(other);
    const { open } = await import("@tauri-apps/plugin-dialog");
    vi.mocked(open).mockResolvedValue("D:/Foto/Roma");
    fireEvent.click(screen.getByRole("button", { name: "Apri cartella…" }));
    await screen.findByRole("heading", { name: "Roma" });
    expect(commandsMock.rawReviewSave).toHaveBeenCalledWith("s1");
    expect(commandsMock.rawReviewClose).toHaveBeenCalledWith("s1");
    expect(commandsMock.rawReviewClose).not.toHaveBeenCalledWith("s2");
  });
});
