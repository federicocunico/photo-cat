import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { usePeopleHotkeys, type PeopleHotkeyActions } from "./usePeopleHotkeys";

function actions(): PeopleHotkeyActions {
  return {
    navigateGroup: vi.fn(),
    toggleFocusedFace: vi.fn(),
    focusWhoIs: vi.fn(),
    focusMove: vi.fn(),
    notSame: vi.fn(),
    merge: vi.fn(),
    ignore: vi.fn(),
    undo: vi.fn(),
  };
}

function Harness({ a, dialog = false }: { a: PeopleHotkeyActions; dialog?: boolean }) {
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
    const a = actions();
    const { getByLabelText } = render(<Harness a={a} />);
    fireEvent.keyDown(getByLabelText("Sposta in"), { key: "ArrowDown" });
    expect(a.navigateGroup).not.toHaveBeenCalled();
    fireEvent.keyDown(document.body, { key: "ArrowDown" });
    expect(a.navigateGroup).toHaveBeenCalledWith(1);
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
