// Barra delle azioni (ui-foundations "Barre dei comandi"; ui-usability-audit 6.1).
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { ActionBar, fitActions, type BarAction } from "./ActionBar";

afterEach(checkedCleanup(A11Y_KNOWN.none));

// Larghezze simulate: la barra misura `width` px, ogni pulsante 8 px per carattere più 32.
let barWidth = 1024;
beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (
    this: HTMLElement,
  ) {
    const w =
      this.getAttribute("role") === "toolbar" ? barWidth : (this.textContent ?? "").length * 8 + 32;
    return {
      width: w,
      height: 32,
      top: 0,
      left: 0,
      right: w,
      bottom: 32,
      x: 0,
      y: 0,
      toJSON() {},
    } as DOMRect;
  });
});
afterEach(() => vi.restoreAllMocks());

const actions = (onSelect = vi.fn()): BarAction[] => [
  { id: "move", label: "Sposta in…", priority: 1, variant: "primary", onSelect },
  { id: "notSame", label: "Non è questa persona", priority: 2, onSelect },
  { id: "newGroup", label: "Nuovo gruppo con questi volti", priority: 3, onSelect },
  { id: "cover", label: "Usa come copertina", priority: 4, onSelect },
  {
    id: "remove",
    label: "Rimuovi volti…",
    priority: 5,
    variant: "danger",
    menuOnly: true,
    onSelect,
  },
];

const renderBar = (width: number) => {
  barWidth = width;
  return render(
    <ActionBar
      label="Volti selezionati"
      lead="3 selezionati"
      actions={actions()}
      end={<button type="button">Deseleziona</button>}
    />,
  );
};

describe("ActionBar", () => {
  it("a 1440 px tutte le azioni sono pulsanti, tranne quelle solo di menu", () => {
    renderBar(1440);
    const bar = screen.getByRole("toolbar", { name: "Volti selezionati" });
    const names = within(bar)
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(names).toEqual([
      "Sposta in…",
      "Non è questa persona",
      "Nuovo gruppo con questi volti",
      "Usa come copertina",
      "Altro",
      "Deseleziona",
    ]);
  });

  it("in una colonna stretta le meno importanti passano in «Altro», senza andare a capo", async () => {
    renderBar(700);
    const bar = screen.getByRole("toolbar", { name: "Volti selezionati" });
    const names = within(bar)
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(names).toEqual(["Sposta in…", "Non è questa persona", "Altro", "Deseleziona"]);
    // Le azioni nascoste sono nel menu Altro, nell'ordine.
    await userEvent.setup().click(within(bar).getByRole("button", { name: "Altro" }));
    const menu = await screen.findByRole("menu");
    expect(
      within(menu)
        .getAllByRole("menuitem")
        .map((m) => m.textContent),
    ).toEqual(["Nuovo gruppo con questi volti", "Usa come copertina", "Rimuovi volti…"]);
  });

  it("a 1024 px la barra resta su una riga: ogni pulsante mostrato ci sta per intero", () => {
    renderBar(1024 - 320 - 300); // finestra minima meno elenco e scheda di Persone
    const bar = screen.getByRole("toolbar", { name: "Volti selezionati" });
    const shown = within(bar).getAllByRole("button");
    const used = shown.reduce((a, b) => a + b.getBoundingClientRect().width + 8, 0);
    expect(used).toBeLessThanOrEqual(1024 - 320 - 300 + 8);
    expect(shown.map((b) => b.textContent)).toContain("Altro");
  });

  it("fitActions: con tutto lo spazio non serve «Altro»", () => {
    expect(fitActions([100, 100], 300, 60)).toBe(2);
    expect(fitActions([100, 100, 100], 300, 60)).toBe(2);
    expect(fitActions([100, 100, 100], 250, 60)).toBe(1);
    expect(fitActions([100], 40, 60)).toBe(0);
  });
});
