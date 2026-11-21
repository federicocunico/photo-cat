import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { RuleEditor } from "./RuleEditor";
import { summarize, type RuleNode } from "./rules";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup(A11Y_KNOWN.none));
beforeEach(() => {
  commandsMock.getTaxonomy.mockResolvedValue({ version: 1, groups: [] } as never);
  commandsMock.libraryFacets.mockResolvedValue(null);
});

function Harness({ initial }: { initial: RuleNode }) {
  const [rule, setRule] = useState(initial);
  return (
    <>
      <RuleEditor rule={rule} onChange={setRule} />
      <output data-testid="json">{JSON.stringify(rule)}</output>
    </>
  );
}

function renderEditor() {
  render(
    <QueryClientProvider client={new QueryClient()}>
      <Harness initial={{ field: "year", cmp: "eq", value: 2020 }} />
    </QueryClientProvider>,
  );
  return () => JSON.parse(screen.getByTestId("json").textContent!) as RuleNode;
}

function choose(select: HTMLElement, name: string | RegExp) {
  const option = within(select).getByRole("option", { name }) as HTMLOptionElement;
  fireEvent.change(select, { target: { value: option.value } });
}

const names = { group: (g: string) => g, label: (_g: string, v: string) => v };

it("campo Valutazione: almeno 4 stelle produce la regola e la frase", () => {
  const json = renderEditor();
  choose(screen.getByLabelText("Campo"), "Valutazione");
  expect(json()).toEqual({ field: "rating", cmp: "eq", value: 3 });
  choose(screen.getByLabelText("Confronto"), "almeno");
  choose(screen.getByLabelText("Valore"), /4 stelle/);
  expect(json()).toEqual({ field: "rating", cmp: "gte", value: 4 });
  const t = i18n.getFixedT("it");
  expect(summarize(t, names, json())).toBe("Foto con valutazione almeno 4 stelle");
});

it("campo Colore: è Bianco e nero produce la regola e la frase", () => {
  const json = renderEditor();
  choose(screen.getByLabelText("Campo"), "Colore");
  expect(json()).toEqual({ field: "color_trait", cmp: "eq", value: "bw" });
  choose(screen.getByLabelText("Valore"), "Seppia");
  expect(json()).toEqual({ field: "color_trait", cmp: "eq", value: "sepia" });
  choose(screen.getByLabelText("Valore"), "Bianco e nero");
  const t = i18n.getFixedT("it");
  expect(summarize(t, names, json())).toBe("Foto con colore Bianco e nero");
  expect(summarize(t, names, { field: "color_trait", cmp: "neq", value: "none" })).toBe(
    "Foto con colore diverso da Non indicato",
  );
});
