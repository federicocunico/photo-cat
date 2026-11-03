import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import {
  clearUndo,
  noteCommand,
  notifyUndoable,
  registerUndo,
  runUndo,
  useUndo,
  useUndoHotkey,
} from "./undo";
import { useToast } from "./toasts";

afterEach(() => {
  cleanup();
  useUndo.setState({ last: null, busy: false });
  useToast.setState({ notices: [] });
});

describe("registro undo", () => {
  it("registra l'operazione e la esegue una sola volta", async () => {
    const run = vi.fn(async () => {});
    registerUndo({ label: "Tag aggiunto", run });
    expect(useUndo.getState().last?.label).toBe("Tag aggiunto");

    expect(await runUndo()).toBe(true);
    expect(run).toHaveBeenCalledTimes(1);
    expect(useUndo.getState().last).toBeNull();
    expect(useToast.getState().notices.map((n) => n.message)).toEqual([
      "Operazione annullata: Tag aggiunto",
    ]);

    expect(await runUndo()).toBe(false);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("una nuova operazione sostituisce la precedente e ne chiude l'avviso", async () => {
    const first = vi.fn(async () => {});
    const second = vi.fn(async () => {});
    const a = notifyUndoable({ message: "Persone unite", run: first });
    notifyUndoable({ message: "Tag rinominato", run: second });

    expect(useToast.getState().notices.map((n) => n.message)).toEqual(["Tag rinominato"]);
    // L'avviso vecchio non può annullare l'operazione nuova.
    expect(await runUndo(a)).toBe(false);
    expect(first).not.toHaveBeenCalled();

    await runUndo();
    expect(second).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
  });

  it("l'azione dell'avviso chiama lo stesso registro", async () => {
    const run = vi.fn(async () => {});
    notifyUndoable({ message: "Persone unite", run });
    const notice = useToast.getState().notices[0];
    expect(notice.action?.label).toBe("Annulla operazione");
    await notice.action?.run();
    expect(run).toHaveBeenCalledTimes(1);
    expect(useUndo.getState().last).toBeNull();
  });

  it("un comando che scrive nel registro del backend fa decadere la voce", () => {
    registerUndo({ label: "x", run: async () => {} });
    noteCommand("listSources");
    expect(useUndo.getState().last).not.toBeNull();
    noteCommand("correctPerson");
    expect(useUndo.getState().last).toBeNull();
  });

  it("durante l'annullamento il proprio comando non fa decadere nulla", async () => {
    let seen: unknown = "unset";
    registerUndo({
      label: "x",
      run: async () => {
        noteCommand("undoBulk");
        seen = useUndo.getState().busy;
      },
    });
    expect(await runUndo()).toBe(true);
    expect(seen).toBe(true);
  });

  it("un errore dell'annullamento diventa un avviso d'errore", async () => {
    registerUndo({
      label: "x",
      run: async () => {
        throw { detail: "Niente da annullare" };
      },
    });
    expect(await runUndo()).toBe(false);
    const [notice] = useToast.getState().notices;
    expect(notice.tone).toBe("error");
    expect(notice.message).toBe("Niente da annullare");
    expect(useUndo.getState().busy).toBe(false);
  });

  it("clearUndo con un id diverso non tocca la voce attuale", () => {
    const id = registerUndo({ label: "x", run: async () => {} });
    clearUndo(id + 100);
    expect(useUndo.getState().last?.id).toBe(id);
    clearUndo(id);
    expect(useUndo.getState().last).toBeNull();
  });
});

function Host() {
  useUndoHotkey();
  return <input aria-label="nome" />;
}

describe("Ctrl+Z", () => {
  it("annulla l'ultima operazione, ma non dentro un campo di testo", async () => {
    const run = vi.fn(async () => {});
    const { getByLabelText } = render(<Host />);
    registerUndo({ label: "x", run });

    fireEvent.keyDown(getByLabelText("nome"), { key: "z", ctrlKey: true });
    expect(run).not.toHaveBeenCalled();

    await act(async () => {
      fireEvent.keyDown(document.body, { key: "z", ctrlKey: true });
    });
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("senza voce registrata lascia il tasto ai moduli", () => {
    render(<Host />);
    const event = new KeyboardEvent("keydown", { key: "z", ctrlKey: true, cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
  });
});
