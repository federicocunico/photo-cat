import { render, screen } from "@testing-library/react";
import "../i18n";
import { SelectionHarness } from "./selectionHarness";
import { runSelectionContract } from "./selectionContract";

const ids = [1, 2, 3, 4, 5, 6, 7, 8];

runSelectionContract({
  name: "raccolta di prova in griglia",
  layout: "grid",
  columns: 4,
  mount: () => {
    render(<SelectionHarness ids={ids} layout="grid" columns={4} withInput />);
  },
  container: () => screen.getByRole("grid"),
  items: () => screen.getAllByRole("gridcell"),
  menuCount: () => {
    const m = screen.queryByRole("menu");
    return m ? Number(/(\d+)/.exec(m.getAttribute("aria-label") ?? "")?.[1]) : null;
  },
  textInput: () => screen.getByRole("textbox", { name: "rinomina" }),
});

runSelectionContract({
  name: "raccolta di prova in elenco",
  layout: "list",
  mount: () => {
    render(<SelectionHarness ids={ids} layout="list" />);
  },
  container: () => screen.getByRole("listbox"),
  items: () => screen.getAllByRole("option"),
});
