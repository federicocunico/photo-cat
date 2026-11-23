// "Unioni recenti" e riepilogo della prima riaggregazione (face-identity-aggregation 5.3, 5.4).
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { MergeEntry } from "../../lib/ipc";
import { ToastHost } from "../../components/feedback/ToastHost";
import { useToast } from "../../store/toasts";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { RecentMerges } from "./RecentMerges";
import { useAggregationSummary } from "./aggregationSummary";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const entry = (id: number, over: Partial<MergeEntry> = {}): MergeEntry => ({
  id,
  at: Date.UTC(2026, 10, 22, 18, 30),
  origin: "auto",
  target: 7,
  targetName: "Gioia",
  targetCover: 70,
  sources: [{ person: 30 + id, name: null, faces: 3, cover: 300 + id }],
  faces: 3,
  undoneAt: null,
  undoable: true,
  ...over,
});

const client = () => new QueryClient({ defaultOptions: { queries: { retry: false } } });

beforeEach(() => {
  vi.clearAllMocks();
  useToast.setState({ notices: [] });
  commandsMock.listMerges.mockResolvedValue([
    entry(1),
    entry(2, {
      origin: "manual",
      target: 8,
      targetName: "Marco",
      sources: [
        { person: 40, name: null, faces: 2, cover: 400 },
        { person: 41, name: null, faces: 5, cover: 410 },
      ],
      faces: 7,
    }),
    entry(3, { origin: "queue", undoneAt: Date.UTC(2026, 10, 23), undoable: false }),
  ]);
  commandsMock.undoMerge.mockResolvedValue(3);
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

describe("Unioni recenti", () => {
  it("Ritrovare un gruppo unito: «Gruppo di 3 volti → Gioia» con origine e Annulla", async () => {
    render(
      <QueryClientProvider client={client()}>
        <RecentMerges open onClose={() => {}} />
        <ToastHost />
      </QueryClientProvider>,
    );
    const list = await screen.findByRole("list", { name: "Unioni recenti" });
    const rows = await within(list).findAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Gruppo di 3 volti → Gioia");
    expect(rows[0]).toHaveTextContent("automatica");
    expect(rows[1]).toHaveTextContent("2 gruppi → Marco");
    expect(rows[1]).toHaveTextContent("a mano");
    expect(rows[2]).toHaveTextContent("dalla coda");
    expect(rows[2]).toHaveTextContent(/Annullata il/);
    expect(within(rows[2]!).queryByRole("button")).toBeNull();
    expect(commandsMock.listMerges).toHaveBeenCalledWith(null, 500);

    fireEvent.click(
      within(rows[0]!).getByRole("button", { name: "Annulla l'unione Gruppo di 3 volti → Gioia" }),
    );
    await waitFor(() => expect(commandsMock.undoMerge).toHaveBeenCalledWith(1));
    expect(
      await screen.findByText(/Unione annullata: 3 volti sono tornati nei gruppi di prima/),
    ).toBeInTheDocument();
  });

  it("il filtro per persona mostra solo le unioni che la riguardano", async () => {
    render(
      <QueryClientProvider client={client()}>
        <RecentMerges open person={8} onClose={() => {}} />
      </QueryClientProvider>,
    );
    const list = await screen.findByRole("list", { name: "Unioni recenti" });
    await waitFor(() => expect(within(list).getAllByRole("listitem")).toHaveLength(1));
    expect(within(list).getByRole("listitem")).toHaveTextContent("Marco");
    fireEvent.change(screen.getByLabelText("Persona"), { target: { value: "" } });
    expect(within(list).getAllByRole("listitem")).toHaveLength(3);
  });
});

function SummaryHost({ onOpen }: { onOpen: () => void }) {
  useAggregationSummary(onOpen);
  return null;
}

describe("Riepilogo della prima riaggregazione", () => {
  it("compare una sola volta, con il collegamento a «Unioni recenti»", async () => {
    commandsMock.aggregationSummary
      .mockResolvedValueOnce({ autoMerges: 52, at: Date.UTC(2026, 10, 23) })
      .mockResolvedValue(null);
    const onOpen = vi.fn();
    const qc = client();
    const view = render(
      <QueryClientProvider client={qc}>
        <SummaryHost onOpen={onOpen} />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(useToast.getState().notices).toHaveLength(1));
    const notice = useToast.getState().notices[0]!;
    expect(notice.message).toBe("52 gruppi uniti automaticamente dopo l'aggiornamento");
    expect(commandsMock.dismissAggregationSummary).toHaveBeenCalledTimes(1);
    await act(async () => notice.action!.run());
    expect(onOpen).toHaveBeenCalledTimes(1);

    // Riaprendo Persone (nuovo montaggio, cache svuotata) il backend non lo ripropone.
    view.unmount();
    useToast.setState({ notices: [] });
    render(
      <QueryClientProvider client={client()}>
        <SummaryHost onOpen={onOpen} />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(commandsMock.aggregationSummary).toHaveBeenCalledTimes(2));
    expect(useToast.getState().notices).toHaveLength(0);
    expect(commandsMock.dismissAggregationSummary).toHaveBeenCalledTimes(1);
  });
});
