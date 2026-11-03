import { act, fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, beforeEach, expect, it } from "vitest";
import "../../i18n";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { useGuide, type TipId } from "../../store/guide";
import { Coachmark } from "./Coachmark";
import { GuideDialog } from "./GuideDialog";

beforeEach(() => useGuide.getState().resetTips());
afterEach(checkedCleanup(A11Y_KNOWN.none));

function Module({ tip }: { tip: TipId }) {
  const anchor = useRef<HTMLHeadingElement>(null);
  return (
    <div>
      <h2 ref={anchor}>{tip}</h2>
      <Coachmark tip={tip} anchor={anchor} learnMore="tags" />
    </div>
  );
}

const TAG_TIP =
  "Un set assegna i tasti 1–9: in Libreria premi un numero per taggare le foto selezionate";

it("Primo ingresso in Tag: il suggerimento si chiude con Ho capito e non ricompare", () => {
  const view = render(<Module tip="tags" />);
  expect(screen.getByText(TAG_TIP)).toBeInTheDocument();
  // Non modale: il focus non si sposta sul suggerimento.
  expect(document.activeElement).toBe(document.body);
  fireEvent.click(screen.getByRole("button", { name: "Ho capito" }));
  expect(screen.queryByText(TAG_TIP)).toBeNull();
  view.unmount();
  render(<Module tip="tags" />);
  expect(screen.queryByText(TAG_TIP)).toBeNull();
});

it("al massimo un suggerimento alla volta; il successivo compare quando il primo si chiude", () => {
  render(
    <>
      <Module tip="tags" />
      <Module tip="sequence" />
    </>,
  );
  expect(screen.getAllByRole("button", { name: "Ho capito" })).toHaveLength(1);
  fireEvent.click(screen.getByRole("button", { name: "Ho capito" }));
  expect(screen.getByText(/Sequenza mette in fila/)).toBeInTheDocument();
});

it("Reimpostare i suggerimenti: ricompaiono al successivo ingresso nel modulo", () => {
  const view = render(<Module tip="tags" />);
  fireEvent.click(screen.getByRole("button", { name: "Ho capito" }));
  view.unmount();
  act(() => useGuide.getState().resetTips());
  render(<Module tip="tags" />);
  expect(screen.getByText(TAG_TIP)).toBeInTheDocument();
});

it("Scopri di più apre la guida sulla sezione del modulo", () => {
  render(
    <>
      <Module tip="tags" />
      <GuideDialog />
    </>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Scopri di più" }));
  expect(screen.getByRole("dialog", { name: "Guida" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Tag" }).closest("section")).toHaveAttribute(
    "data-current",
    "true",
  );
});
