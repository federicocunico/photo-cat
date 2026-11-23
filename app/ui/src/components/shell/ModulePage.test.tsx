// Intestazione di pagina e scorrimento (desktop-shell, "Intestazione di pagina" e "Scorrimento e
// barra di stato"; ui-usability-audit 3.4, 3.5): ogni modulo usa ModulePage, il titolo sta sempre
// nello stesso contenitore e ha la stessa altezza, e scorre solo il corpo del modulo.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ModulePage } from "./ModulePage";

afterEach(cleanup);

const sources = import.meta.glob(["../../**/*.tsx", "!../../**/*.test.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;
const source = (file: string) => Object.entries(sources).find(([k]) => k.endsWith(file))?.[1] ?? "";

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
// Fine riga normalizzati: in un checkout Windows (core.autocrlf) i CSS hanno CRLF.
const css = (file: string) =>
  fs.readFileSync(path.join(SRC, file), "utf8").replace(/\r\n/g, "\n");
const rule = (text: string, selector: string) =>
  new RegExp(`(^|\\n)${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`).exec(
    text,
  )?.[2] ?? "";

/** Le pagine dei moduli di primo livello (ReadyApp → ModuleView). */
const MODULE_PAGES = [
  "modules/library/LibraryPage.tsx",
  "modules/import/ImportPage.tsx",
  "modules/jobs/JobsPage.tsx",
  "modules/people/PeopleModule.tsx",
  "modules/catalogs/CatalogsPage.tsx",
  "modules/tags/TagsPage.tsx",
  "modules/sequence/SequencePage.tsx",
  "modules/organize/OrganizePage.tsx",
  "modules/raw-review/RawReviewPage.tsx",
  "modules/ai/AiPage.tsx",
  "modules/settings/SettingsPage.tsx",
];

describe("intestazione di pagina", () => {
  it("ogni modulo usa ModulePage e nessuno costruisce l'intestazione da sé", () => {
    for (const file of MODULE_PAGES) expect(source(file), file).toContain("<ModulePage");
    const direct = Object.entries(sources)
      .filter(([k, v]) => !k.endsWith("ModulePage.tsx") && /<PageHeader\b/.test(v))
      .map(([k]) => k);
    expect(direct).toEqual([]);
  });

  it("titolo, sottotitolo, schede e comandi stanno nell'intestazione, il corpo sotto", () => {
    render(
      <ModulePage
        title="Persone"
        subtitle="Fujifilm X-T5"
        tabs={<div role="tablist" aria-label="Sezioni" />}
        actions={<button type="button">Nuovo catalogo</button>}
      >
        <p>contenuto</p>
      </ModulePage>,
    );
    const header = screen.getByRole("heading", { level: 1, name: "Persone" }).closest("header")!;
    expect(header).toHaveClass("ui-page-header");
    expect(header.parentElement).toHaveClass("module-page");
    expect(header).toContainElement(screen.getByText("Fujifilm X-T5"));
    expect(header).toContainElement(screen.getByRole("tablist"));
    expect(header).toContainElement(screen.getByRole("button", { name: "Nuovo catalogo" }));
    const body = screen.getByText("contenuto").parentElement!;
    expect(body).toHaveClass("module-body", "module-body--scroll");
    expect(header.nextElementSibling).toBe(body);
  });

  it("con scroll={false} il corpo non scorre: lo fanno i riquadri del modulo", () => {
    render(
      <ModulePage title="Libreria" scroll={false}>
        <p>griglia</p>
      </ModulePage>,
    );
    expect(screen.getByText("griglia").parentElement).not.toHaveClass("module-body--scroll");
  });

  it("l'intestazione ha altezza fissa e la pagina non aggiunge margini", () => {
    const header = rule(css("components/ui/ui.css"), ".ui-page-header");
    expect(header).toMatch(/height:\s*56px/);
    expect(header).not.toMatch(/flex-wrap:\s*wrap/);
    const page = rule(css("components/shell/shell.css"), ".module-page");
    expect(page).not.toMatch(/padding|margin/);
  });
});

describe("scorrimento", () => {
  it("la finestra non scorre: html, body e la colonna principale nascondono l'eccedenza", () => {
    expect(rule(css("styles/global.css"), "html,\nbody,\n#root")).toMatch(/overflow:\s*hidden/);
    const main = rule(css("components/shell/shell.css"), ".app-shell-main");
    expect(main).toMatch(/min-height:\s*0/);
    expect(main).toMatch(/overflow:\s*hidden/);
    expect(rule(css("components/shell/shell.css"), ".module-body--scroll")).toMatch(
      /overflow-y:\s*auto/,
    );
  });

  it("F6: la barra di stato resta in fondo, su una riga", () => {
    const bar = rule(css("components/shell/shell.css"), ".status-bar");
    expect(bar).toMatch(/flex:\s*none/);
    expect(bar).toMatch(/flex-wrap:\s*nowrap/);
    expect(bar).toMatch(/white-space:\s*nowrap/);
    expect(bar).toMatch(/overflow:\s*hidden/);
  });

  it("F5: Esporta scorre nel corpo del modulo, non con un contenitore proprio", () => {
    const organize = source("modules/organize/OrganizePage.tsx");
    expect(organize).toMatch(/<ModulePage[^>]*bodyClassName="export-body"/);
    expect(organize).not.toMatch(/scroll=\{false\}/);
    expect(css("modules/organize/organize.css")).not.toMatch(/\.export-page\s*\{/);
  });
});
