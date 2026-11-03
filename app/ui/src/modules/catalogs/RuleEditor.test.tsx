import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import i18n from "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { RuleEditor } from "./RuleEditor";
import { MB, summarize, type RuleNode } from "./rules";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(checkedCleanup(A11Y_KNOWN.none));

// Come in i18n/glossary.test.ts, più gli altri confronti: identificatori interni che l'utente
// non deve vedere. I campi si controllano sui valori delle opzioni (le icone SVG hanno
// attributi "width" e "height").
const JARGON = /\b(eq|neq|gt|gte|lt|lte|contains|people_count)\b/;
const IDS = ["tag", "person", "place", "format", "year", "people_count", "width", "height"];
const MORE_IDS = ["size", "name", "status", "label.subject", "eq", "gte", "contains"];

const taxonomy = {
  version: 1,
  groups: [
    {
      id: "subject",
      label: "Tipo soggetto",
      threshold: 0.25,
      categories: [
        { id: "ritratto", label: "Ritratto", enabled: true, prompts: [] },
        { id: "paesaggio", label: "Paesaggio", enabled: true, prompts: [] },
      ],
    },
  ],
};

beforeEach(() => {
  commandsMock.getTaxonomy.mockResolvedValue(taxonomy as never);
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

function renderEditor(initial: RuleNode) {
  const view = render(
    <QueryClientProvider client={new QueryClient()}>
      <Harness initial={initial} />
    </QueryClientProvider>,
  );
  const json = () => JSON.parse(screen.getByTestId("json").textContent!) as RuleNode;
  // Il DOM dell'editor, senza l'uscita di prova con il JSON.
  const editor = () => view.container.querySelector("fieldset")!;
  return { json, editor };
}

/** Sceglie un'opzione per etichetta (i valori delle opzioni non sono gli id interni). */
function choose(select: HTMLElement, name: string) {
  const option = within(select).getByRole("option", { name }) as HTMLOptionElement;
  fireEvent.change(select, { target: { value: option.value } });
}

const every: RuleNode = {
  op: "and",
  rules: [
    { field: "label.subject", cmp: "eq", value: "ritratto" },
    { field: "people_count", cmp: "gte", value: 1 },
    { field: "year", cmp: "gte", value: 2022 },
    { field: "tag", cmp: "neq", value: "Scarti" },
    {
      op: "or",
      rules: [
        { field: "width", cmp: "gt", value: 4000 },
        { field: "height", cmp: "lte", value: 3000 },
        { field: "size", cmp: "lt", value: 20 * MB },
        { field: "status", cmp: "eq", value: "missing" },
        { field: "name", cmp: "contains", value: "IMG" },
        { field: "format", cmp: "eq", value: "jpg" },
        { field: "place", cmp: "neq", value: "Roma" },
        { field: "person", cmp: "eq", value: "Anna" },
      ],
    },
  ],
};

it("nessun id interno di campi o confronti nel DOM dell'editor", async () => {
  const { editor } = renderEditor(every);
  await screen.findByRole("option", { name: "Ritratto" });
  const html = editor().outerHTML;
  // Né nel testo né negli attributi (valori delle opzioni compresi).
  expect(html).not.toMatch(JARGON);
  expect(editor().textContent).not.toMatch(/people_count|label\.subject/);
  const values = [...editor().querySelectorAll("option")].map((o) => o.value);
  for (const id of [...IDS, ...MORE_IDS]) expect(values).not.toContain(id);
  expect(screen.getAllByRole("option", { name: "Persone nella foto" }).length).toBeGreaterThan(0);
  expect(screen.getAllByRole("option", { name: "Dimensione del file" }).length).toBeGreaterThan(0);
  // Valori con l'unità: persone, pixel, MB; stato in parole.
  expect(screen.getByText("persona")).toBeInTheDocument();
  expect(screen.getAllByText("px")).toHaveLength(2);
  expect(screen.getByText("MB")).toBeInTheDocument();
  expect(screen.getByDisplayValue("20")).toBeInTheDocument();
  expect(screen.getByRole("option", { name: "Mancante", selected: true })).toBeInTheDocument();
  expect(screen.getByDisplayValue("JPG")).toBeInTheDocument();
});

it("Nuova regola: «Persone nella foto» offre i confronti numerici e un valore numerico", async () => {
  const { json } = renderEditor({ op: "and", rules: [{ field: "tag", cmp: "eq", value: "" }] });
  // Il tag accetta solo "è" e "non è" (il backend rifiuta "contiene").
  const cmp = screen.getByRole("combobox", { name: "Confronto" });
  expect(
    within(cmp)
      .getAllByRole("option")
      .map((o) => o.textContent),
  ).toEqual(["è", "non è"]);
  choose(screen.getByRole("combobox", { name: "Campo" }), "Persone nella foto");
  expect(
    within(screen.getByRole("combobox", { name: "Confronto" }))
      .getAllByRole("option")
      .map((o) => o.textContent),
  ).toEqual(["è", "non è", "più di", "almeno", "meno di", "al massimo"]);
  const value = screen.getByRole("spinbutton", { name: "Valore" });
  fireEvent.change(value, { target: { value: "3" } });
  choose(screen.getByRole("combobox", { name: "Confronto" }), "almeno");
  expect(json()).toEqual({
    op: "and",
    rules: [{ field: "people_count", cmp: "gte", value: 3 }],
  });
  expect(screen.getByText("persone")).toBeInTheDocument();
});

it("la dimensione si scrive in MB e si salva in byte", () => {
  const { json } = renderEditor({ op: "and", rules: [{ field: "size", cmp: "gt", value: 0 }] });
  fireEvent.change(screen.getByRole("spinbutton", { name: "Valore" }), { target: { value: "5" } });
  expect(json()).toEqual({ op: "and", rules: [{ field: "size", cmp: "gt", value: 5 * MB }] });
});

it("Riepilogo della regola: una frase nella lingua dell'interfaccia", () => {
  const t = i18n.getFixedT("it");
  const names = {
    group: (g: string) => (g === "subject" ? "Tipo soggetto" : g),
    label: (_: string, v: string) => (v === "ritratto" ? "Ritratto" : v),
  };
  const rule: RuleNode = {
    op: "and",
    rules: [
      { field: "label.subject", cmp: "eq", value: "ritratto" },
      { field: "year", cmp: "gte", value: 2022 },
    ],
  };
  expect(summarize(t, names, rule)).toBe("Foto con tipo soggetto Ritratto, dal 2022 in poi");
  expect(summarize(t, names, every)).not.toMatch(JARGON);
  expect(
    summarize(t, names, {
      op: "or",
      rules: [
        { field: "tag", cmp: "eq", value: "Mare" },
        { field: "people_count", cmp: "gte", value: 2 },
      ],
    }),
  ).toBe("Foto con il tag «Mare» o con persone nella foto almeno 2 persone");
});
