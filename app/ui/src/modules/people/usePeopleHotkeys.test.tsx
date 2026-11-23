import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { useRef } from "react";
import { usePeopleHotkeys, type PeopleHotkeyActions } from "./usePeopleHotkeys";
import { useActivePane, type ActivePane } from "../../commands/gridShortcuts";

function actions(): PeopleHotkeyActions {
  return {
    toggleFocusedFace: vi.fn(),
    focusWhoIs: vi.fn(),
    focusMove: vi.fn(),
    notSame: vi.fn(),
    merge: vi.fn(),
    ignore: vi.fn(),
    undo: vi.fn(),
  };
}

const pane = (): ActivePane => ({
  selectAll: vi.fn(),
  invert: vi.fn(),
  clear: vi.fn(),
  count: () => 0,
});

/** Un riquadro con una raccolta focalizzabile che registra i tasti ricevuti. */
function Pane({ name, keys }: { name: string; keys: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  useActivePane(pane(), ref);
  return (
    <div ref={ref}>
      <div
        role="listbox"
        aria-label={name}
        tabIndex={0}
        onKeyDown={(e) => {
          keys.push(`${name}:${e.shiftKey ? "Maiusc+" : ""}${e.key}`);
          e.preventDefault();
        }}
      >
        <div role="option" aria-selected={false}>
          {name}
        </div>
      </div>
    </div>
  );
}

function Harness({
  a,
  dialog = false,
  keys = [],
}: {
  a: PeopleHotkeyActions;
  dialog?: boolean;
  keys?: string[];
}) {
  usePeopleHotkeys(a);
  return (
    <div>
      <button type="button">Unisci</button>
      <input type="checkbox" aria-label="Scegli" />
      <button type="button" data-face-tile>
        volto
      </button>
      <select aria-label="Sposta in">
        <option>a</option>
      </select>
      <Pane name="elenco" keys={keys} />
      <Pane name="volti" keys={keys} />
      {dialog && (
        <div role="dialog" aria-modal="true">
          finestra
        </div>
      )}
    </div>
  );
}

afterEach(cleanup);

describe("usePeopleHotkeys", () => {
  it("lascia Spazio ai pulsanti e alle caselle, ma lo usa sulle miniature dei volti", () => {
    const a = actions();
    const { getByText, getByLabelText } = render(<Harness a={a} />);
    fireEvent.keyDown(getByText("Unisci"), { key: " " });
    fireEvent.keyDown(getByLabelText("Scegli"), { key: " " });
    expect(a.toggleFocusedFace).not.toHaveBeenCalled();
    fireEvent.keyDown(getByText("volto"), { key: " " });
    expect(a.toggleFocusedFace).toHaveBeenCalledTimes(1);
  });

  it("lascia le frecce al selettore", () => {
    const keys: string[] = [];
    const { getByLabelText } = render(<Harness a={actions()} keys={keys} />);
    fireEvent.keyDown(getByLabelText("Sposta in"), { key: "ArrowDown" });
    expect(keys).toEqual([]);
  });

  it("fuori da un elenco le frecce vanno al riquadro attivo, con i modificatori", () => {
    const keys: string[] = [];
    const { getByText, getByRole } = render(<Harness a={actions()} keys={keys} />);
    // Ultimo clic sui volti: la freccia giù (con il focus su un pulsante) si muove tra i volti.
    fireEvent.mouseDown(getByText("volti"));
    getByText("Unisci").focus();
    fireEvent.keyDown(getByText("Unisci"), { key: "ArrowDown" });
    expect(keys).toEqual(["volti:ArrowDown"]);
    expect(document.activeElement).toBe(getByRole("listbox", { name: "volti" }));
    // Poi sull'elenco: Maiusc+su va all'elenco, non ai volti.
    fireEvent.mouseDown(getByText("elenco"));
    fireEvent.keyDown(document.body, { key: "ArrowUp", shiftKey: true });
    expect(keys).toEqual(["volti:ArrowDown", "elenco:Maiusc+ArrowUp"]);
  });

  it("si sospende con una finestra modale aperta", () => {
    const a = actions();
    render(<Harness a={a} dialog />);
    fireEvent.keyDown(document.body, { key: "u" });
    fireEvent.keyDown(document.body, { key: "z", ctrlKey: true });
    expect(a.merge).not.toHaveBeenCalled();
    expect(a.undo).not.toHaveBeenCalled();
  });
});
