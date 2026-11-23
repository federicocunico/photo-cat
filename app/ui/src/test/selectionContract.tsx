// Contratto di test del modello di selezione (unified-selection, design D8): gli scenari della
// spec `item-selection` eseguiti su una raccolta montata. Ogni raccolta lo chiama nel proprio file
// di test, così non può comportarsi diversamente dalle altre senza far fallire i test.
// Usa solo ruoli e attributi ARIA (`aria-selected`, `aria-activedescendant`) e gesti reali.
import { act, cleanup, fireEvent, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

export interface ContractTarget {
  /** Nome della raccolta nei titoli dei test. */
  name: string;
  layout: "grid" | "list";
  /** Colonne visibili in griglia (per le frecce su e giù). */
  columns?: number;
  /** Monta la raccolta con almeno 6 elementi e attende che siano visibili. */
  mount: () => Promise<void> | void;
  /** Contenitore che riceve i tasti (ruolo grid o listbox). */
  container: () => HTMLElement;
  /** Elementi nell'ordine mostrato. */
  items: () => HTMLElement[];
  /** Selezionato (predefinito: `aria-selected="true"`). */
  isSelected?: (el: HTMLElement) => boolean;
  /** Indice dell'elemento con il focus (predefinito: da `aria-activedescendant`). */
  focusedIndex?: () => number;
  /** Elemento da cliccare come "spazio vuoto" (predefinito: il contenitore). */
  emptyArea?: () => HTMLElement;
  /** Elemento su cui fare clic per selezionare (predefinito: l'elemento stesso). */
  clickTarget?: (item: HTMLElement) => HTMLElement;
  /** Numero di elementi su cui agisce il menu contestuale aperto, se la raccolta lo espone. */
  menuCount?: () => number | null;
  /** Campo di testo nella schermata della raccolta, se esiste. */
  textInput?: () => HTMLElement | null;
}

export function runSelectionContract(t: ContractTarget) {
  const selected = () =>
    t
      .items()
      .map((el, i) => ((t.isSelected ?? aria)(el) ? i : -1))
      .filter((i) => i >= 0);
  const focused = () => {
    if (t.focusedIndex) return t.focusedIndex();
    const id = t.container().getAttribute("aria-activedescendant");
    return t.items().findIndex((el) => el.id === id);
  };
  const click = (i: number, mods: { ctrlKey?: boolean; shiftKey?: boolean } = {}) =>
    act(() => {
      const el = t.items()[i]!;
      fireEvent.click(t.clickTarget?.(el) ?? el, mods);
    });
  const key = (k: string, mods: { ctrlKey?: boolean; shiftKey?: boolean } = {}) =>
    act(() => {
      fireEvent.keyDown(t.container(), { key: k, ...mods });
    });
  const down = t.layout === "grid" ? "ArrowRight" : "ArrowDown";
  const up = t.layout === "grid" ? "ArrowLeft" : "ArrowUp";

  describe(`contratto di selezione: ${t.name}`, () => {
    afterEach(cleanup);

    it("clic, Ctrl+clic, Maiusc+clic e Ctrl+Maiusc+clic", async () => {
      await t.mount();
      click(1);
      expect(selected()).toEqual([1]);
      click(3, { shiftKey: true });
      expect(selected()).toEqual([1, 2, 3]);
      click(2, { ctrlKey: true });
      expect(selected()).toEqual([1, 3]);
      click(0);
      click(2, { ctrlKey: true });
      click(4, { ctrlKey: true, shiftKey: true });
      expect(selected()).toEqual([0, 2, 3, 4]);
    });

    it("clic su uno spazio vuoto svuota la selezione", async () => {
      await t.mount();
      click(0);
      click(2, { shiftKey: true });
      act(() => {
        fireEvent.click(t.emptyArea?.() ?? t.container());
      });
      expect(selected()).toEqual([]);
    });

    it("le frecce spostano il focus e selezionano solo l'elemento raggiunto", async () => {
      await t.mount();
      click(1);
      key(down);
      expect(selected()).toEqual([2]);
      expect(focused()).toBe(2);
      key(up);
      expect(selected()).toEqual([1]);
    });

    it("Maiusc estende dall'ancora, Ctrl sposta solo il focus, Ctrl+Spazio aggiunge", async () => {
      await t.mount();
      click(1);
      key(down, { shiftKey: true });
      key(down, { shiftKey: true });
      expect(selected()).toEqual([1, 2, 3]);
      click(0);
      key(down, { ctrlKey: true });
      key(down, { ctrlKey: true });
      expect(selected()).toEqual([0]);
      expect(focused()).toBe(2);
      key(" ", { ctrlKey: true });
      expect(selected()).toEqual([0, 2]);
    });

    it("Home e Fine selezionano il primo e l'ultimo elemento", async () => {
      await t.mount();
      click(2);
      key("End");
      expect(selected()).toEqual([t.items().length - 1]);
      key("Home");
      expect(selected()).toEqual([0]);
    });

    it("Ctrl+A seleziona tutto, Ctrl+Maiusc+I inverte, Esc svuota", async () => {
      await t.mount();
      click(1);
      key("a", { ctrlKey: true });
      await waitFor(() => expect(selected()).toHaveLength(t.items().length));
      key("Escape");
      await waitFor(() => expect(selected()).toEqual([]));
      click(0);
      key("I", { ctrlKey: true, shiftKey: true });
      await waitFor(() =>
        expect(selected()).toEqual(
          t
            .items()
            .slice(1)
            .map((_, i) => i + 1),
        ),
      );
    });

    it("clic destro: su un elemento selezionato tiene la selezione, altrimenti seleziona solo lui", async () => {
      await t.mount();
      click(0);
      click(2, { shiftKey: true });
      act(() => {
        fireEvent.contextMenu(t.clickTarget?.(t.items()[1]!) ?? t.items()[1]!);
      });
      expect(selected()).toEqual([0, 1, 2]);
      if (t.menuCount) expect(t.menuCount()).toBe(3);
      act(() => {
        fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
      });
      act(() => {
        fireEvent.contextMenu(t.clickTarget?.(t.items()[4]!) ?? t.items()[4]!);
      });
      expect(selected()).toEqual([4]);
    });

    it("nessuna scorciatoia di selezione in un campo di testo", async () => {
      await t.mount();
      const input = t.textInput?.();
      if (!input) return;
      click(1);
      act(() => {
        fireEvent.keyDown(input, { key: "a", ctrlKey: true });
        fireEvent.keyDown(input, { key: down });
      });
      expect(selected()).toEqual([1]);
    });
  });
}

const aria = (el: HTMLElement) => el.getAttribute("aria-selected") === "true";
