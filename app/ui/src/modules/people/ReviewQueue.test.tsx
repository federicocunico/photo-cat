// Coda "Da confermare" (face-identity-aggregation 6.3): scenari di "Coda Da confermare" e
// "Propagazione delle risposte" lato interfaccia.
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import type { QuestionGroup, ReviewQuestion, ReviewQueue } from "../../lib/ipc";
import { ToastHost } from "../../components/feedback/ToastHost";
import { useUndo } from "../../store/undo";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { ReviewQueuePanel } from "./ReviewQueue";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (k: string) => `http://thumb.localhost/${k}`,
}));

const grp = (id: number, name: string | null, faces = 3): QuestionGroup => ({
  id,
  name,
  faces,
  photos: faces,
  cover: id * 10,
});
const GIOIA = grp(7, "Gioia", 40);
const question = (id: number, over: Partial<ReviewQuestion> = {}): ReviewQuestion => ({
  id,
  kind: "group_person",
  group: grp(100 + id, null),
  other: GIOIA,
  second: null,
  probability: 0.9,
  ...over,
});
const queueOf = (items: ReviewQueue["items"], open = items.length): ReviewQueue => ({
  items,
  open,
  unnamedGroups: 37,
});
const outcome = { mergedGroups: 1, faces: 3, constrainedGroups: 0, conflict: null };

function renderQueue() {
  return render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <ReviewQueuePanel />
      <ToastHost />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useUndo.setState({ last: null, busy: false });
  commandsMock.groupFaces.mockResolvedValue({
    items: [],
    next: null,
    total: 0,
    goodCount: 0,
    discardedCount: 0,
  } as never);
  commandsMock.answerQuestion.mockResolvedValue(outcome);
  commandsMock.answerQuestions.mockResolvedValue({
    ...outcome,
    mergedGroups: 10,
    constrainedGroups: 2,
  });
});
afterEach(checkedCleanup(A11Y_KNOWN.people));

describe("Coda Da confermare", () => {
  it("«È Gioia?»: Sì (anche da tastiera) unisce con un'operazione annullabile", async () => {
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([{ kind: "single", question: question(1) }], 8),
    );
    renderQueue();
    const q = await screen.findByRole("region", { name: "È Gioia?" });
    expect(within(q).getByText("molto probabile")).toBeInTheDocument();
    expect(screen.getByText("1 di 8")).toBeInTheDocument();
    // Da tastiera: S = Sì.
    fireEvent.keyDown(screen.getByRole("region", { name: "Da confermare" }), { key: "s" });
    await waitFor(() =>
      expect(commandsMock.answerQuestion).toHaveBeenCalledWith(1, { kind: "yes" }, false),
    );
    expect(await screen.findByText("Gruppo unito a Gioia")).toBeInTheDocument();
    expect(useUndo.getState().last?.label).toBe("Gruppo unito a Gioia");
    expect(commandsMock.reviewQueue.mock.calls.length).toBeGreaterThan(1);
  });

  it("«No» diventa un vincolo", async () => {
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([{ kind: "single", question: question(2) }]),
    );
    commandsMock.answerQuestion.mockResolvedValue({
      ...outcome,
      mergedGroups: 0,
      constrainedGroups: 1,
    });
    renderQueue();
    const q = await screen.findByRole("region", { name: "È Gioia?" });
    fireEvent.click(within(q).getByRole("button", { name: "No" }));
    await waitFor(() =>
      expect(commandsMock.answerQuestion).toHaveBeenCalledWith(2, { kind: "no" }, false),
    );
    expect(await screen.findByText("Segnati come persone diverse")).toBeInTheDocument();
  });

  it("«È Anna o Marta?»: si sceglie una delle due o nessuna", async () => {
    const anna = grp(8, "Anna", 20);
    const marta = grp(9, "Marta", 25);
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([
        {
          kind: "single",
          question: question(3, { kind: "group_choice", other: anna, second: marta }),
        },
      ]),
    );
    renderQueue();
    const q = await screen.findByRole("region", { name: "È Anna o Marta?" });
    expect(within(q).getByRole("button", { name: "Nessuna delle due" })).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole("region", { name: "Da confermare" }), { key: "2" });
    await waitFor(() =>
      expect(commandsMock.answerQuestion).toHaveBeenCalledWith(
        3,
        { kind: "choose", person: 9 },
        false,
      ),
    );
  });

  it("Conferma in blocco: i gruppi tolti ricevono il vincolo, gli altri passano a Gioia", async () => {
    const questions = [1, 2, 3, 4].map((i) => question(i));
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([{ kind: "batch", person: GIOIA, questions }]),
    );
    renderQueue();
    const block = await screen.findByRole("region", { name: "4 gruppi probabilmente di Gioia" });
    const boxes = within(block).getAllByRole("checkbox");
    expect(boxes.every((b) => (b as HTMLInputElement).checked)).toBe(true);
    fireEvent.click(boxes[1]!);
    fireEvent.click(boxes[3]!);
    fireEvent.click(within(block).getByRole("button", { name: "Conferma tutti (2)" }));
    await waitFor(() =>
      expect(commandsMock.answerQuestions).toHaveBeenCalledWith(7, [1, 3], [2, 4], false),
    );
    expect(await screen.findByText("10 gruppi uniti a Gioia")).toBeInTheDocument();
  });

  it("Conflitto: nessuna unione finché non si sceglie di togliere il vincolo", async () => {
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([{ kind: "single", question: question(5) }]),
    );
    commandsMock.answerQuestion
      .mockResolvedValueOnce({
        ...outcome,
        mergedGroups: 0,
        conflict: { question: 5, group: 105, person: 7 },
      })
      .mockResolvedValueOnce(outcome);
    renderQueue();
    const q = await screen.findByRole("region", { name: "È Gioia?" });
    fireEvent.click(within(q).getByRole("button", { name: "Sì, è Gioia" }));
    expect(await screen.findByText("Risposta in conflitto")).toBeInTheDocument();
    expect(useUndo.getState().last).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Togli l'indicazione e unisci" }));
    await waitFor(() =>
      expect(commandsMock.answerQuestion).toHaveBeenLastCalledWith(5, { kind: "yes" }, true),
    );
    expect(await screen.findByText("Gruppo unito a Gioia")).toBeInTheDocument();
  });

  it("Coda vuota: lo dice e indica i gruppi senza nome rimasti", async () => {
    commandsMock.reviewQueue.mockResolvedValue(queueOf([], 0));
    renderQueue();
    expect(
      await screen.findByText("Niente da confermare. Restano 37 gruppi senza nome in Da rivedere."),
    ).toBeInTheDocument();
  });

  it("→ salta alla domanda successiva senza rispondere", async () => {
    commandsMock.reviewQueue.mockResolvedValue(
      queueOf([
        { kind: "single", question: question(1) },
        {
          kind: "single",
          question: question(2, { kind: "group_group", other: grp(50, null) }),
        },
      ]),
    );
    renderQueue();
    await screen.findByRole("region", { name: "È Gioia?" });
    fireEvent.keyDown(screen.getByRole("region", { name: "Da confermare" }), { key: "ArrowRight" });
    expect(await screen.findByRole("region", { name: "È la stessa persona?" })).toBeInTheDocument();
    expect(commandsMock.answerQuestion).not.toHaveBeenCalled();
  });
});
