// Scala tipografica fissa (desktop-shell, "Stile visivo"; ui-usability-audit D10): solo i corpi
// 12, 14, 16 e 20 px, i token corrispondenti e nessun testo in maiuscolo forzato.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { scales } from "../theme/tokens";

// Con `css: false` Vitest non restituisce il testo dei CSS importati: si leggono dal disco.
const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
function readCss(dir: string, out: Record<string, string> = {}) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) readCss(p, out);
    else if (e.name.endsWith(".css"))
      out[path.relative(SRC, p).replace(/\\/g, "/")] = fs.readFileSync(p, "utf8");
  }
  return out;
}
const css = readCss(SRC);
const tsx = import.meta.glob(["../**/*.tsx", "!../**/*.test.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const SIZES = new Set([12, 14, 16, 20]);
const TOKENS = new Set(["sm", "base", "lg", "xl"]);

/** Ogni corpo fuori scala come "file: dichiarazione". */
function offScale(): string[] {
  const out: string[] = [];
  for (const [file, text] of Object.entries(css)) {
    for (const m of text.matchAll(/font-size:\s*([^;}]+)/g)) {
      const v = m[1]!.trim();
      const px = /^(\d+)px$/.exec(v);
      const token = /^var\(--pc-font-([a-z]+)\)$/.exec(v);
      if (v === "inherit" || (px && SIZES.has(Number(px[1]))) || (token && TOKENS.has(token[1]!)))
        continue;
      out.push(`${file}: font-size: ${v}`);
    }
    for (const m of text.matchAll(/(?:^|[;{\s])font:\s*([^;}]+)/g)) {
      for (const part of m[1]!.trim().split(/\s+/)) {
        const px = /^(\d+)px(\/.*)?$/.exec(part);
        if (px && !SIZES.has(Number(px[1]))) out.push(`${file}: font: ${m[1]}`);
      }
    }
    for (const m of text.matchAll(/--pc-font-([a-z]+)\b/g))
      if (!["ui", "mono"].includes(m[1]!) && !TOKENS.has(m[1]!))
        out.push(`${file}: --pc-font-${m[1]}`);
  }
  for (const [file, text] of Object.entries(tsx))
    for (const m of text.matchAll(/fontSize:\s*(\d+)\b/g))
      if (!SIZES.has(Number(m[1]))) out.push(`${file}: fontSize: ${m[1]}`);
  return out;
}

describe("scala tipografica", () => {
  it("i token sono i quattro corpi 12, 14, 16 e 20 px", () => {
    expect(Object.values(scales.font).map((v) => Number.parseInt(v, 10))).toEqual([12, 14, 16, 20]);
  });

  it("nessun corpo fuori scala nei CSS e negli stili in linea", () => {
    expect(offScale()).toEqual([]);
  });

  it("nessun testo in maiuscolo forzato", () => {
    const found = [
      ...Object.entries(css).filter(([, t]) => /text-transform:\s*uppercase/.test(t)),
      ...Object.entries(tsx).filter(([, t]) => /textTransform:\s*["']uppercase/.test(t)),
    ].map(([f]) => f);
    expect(found).toEqual([]);
  });

  it("monospazio solo per percorsi e codici, mai per conteggi, date e nomi di file", () => {
    // File con percorsi, testo tecnico, tasti o modelli di nome: gli unici che possono usarlo.
    const ALLOWED = new Set([
      "components/about/AboutPanel.tsx",
      "components/feedback/BootScreen.tsx",
      "components/shell/AppShell.tsx",
      "components/shell/TopBar.tsx",
      "modules/diagnostics/DiagnosticsPage.tsx",
      "modules/diagnostics/ErrorGroups.tsx",
      "modules/import/ImportPage.tsx",
      "modules/import/manage.tsx",
      "modules/jobs/JobsPage.tsx",
      "modules/library/Inspector.tsx",
      "modules/library/Sidebar.tsx",
      "modules/models/ModelsPage.tsx",
      "modules/organize/OrganizePage.tsx",
      "modules/raw-review/ExportToCatalogDialog.tsx",
      "modules/sequence/SequenceExport.tsx",
      "modules/sequence/SequencePage.tsx",
      "modules/settings/ExternalAppsSection.tsx",
      "modules/settings/GeneralSection.tsx",
      "modules/tags/TagsPage.tsx",
    ]);
    const NUMBERS = /fmt(Int|Dec|Date|DateTime|Time|Bytes|Duration|Eta)\(|%`|\bpct\b|count:/;
    const found: string[] = [];
    for (const [key, text] of Object.entries(tsx)) {
      const file = key.replace(/^\.\.\//, "");
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (!/className=[{"`][^>]*\bmono\b/.test(line)) return;
        // Registri e testo tecnico (<pre>) restano in monospazio anche con gli orari.
        if (/<pre\b/.test(line)) return;
        // Contenuto dell'elemento: dalla riga con la classe al primo tag di chiusura.
        const rest = lines.slice(i, i + 4).join(" ");
        const near = rest.slice(0, rest.indexOf("</") + 1 || undefined);
        if (!ALLOWED.has(file) || NUMBERS.test(near)) found.push(`${file}:${i + 1}`);
      });
    }
    expect(found).toEqual([]);
  });

  it("E5: in Persone gli stati (selezionato, aperto, attivo) vengono dalle classi, non da stili in linea", () => {
    const found = Object.entries(tsx)
      .filter(([f]) => f.includes("/modules/people/"))
      .flatMap(([f, text]) =>
        [...text.matchAll(/style=\{\{([^}]*)\}\}/g)]
          .filter((m) => /\b(border|borderColor|background|opacity)\s*:/.test(m[1]!))
          .map((m) => `${f}: ${m[1]!.trim().slice(0, 60)}`),
      );
    expect(found).toEqual([]);
  });

  it("un solo stile per le intestazioni di sezione: 16 px, peso 600", () => {
    const globalFile = Object.keys(css).find((f) => f.endsWith("styles/global.css"))!;
    const global = css[globalFile]!;
    const rule = /\.section-title\s*\{([^}]*)\}/.exec(global)?.[1] ?? "";
    expect(rule).toContain("var(--pc-weight-medium) var(--pc-font-lg)");
    // Nessun altro file ridefinisce il corpo di .section-title.
    const others = Object.entries(css).filter(
      ([f, t]) => f !== globalFile && /\.section-title\s*\{[^}]*font/.test(t),
    );
    expect(others.map(([f]) => f)).toEqual([]);
  });
});
