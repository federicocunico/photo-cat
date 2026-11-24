// L'intestazione dei volti e la barra delle azioni sui volti si danno il cambio nello stesso posto
// quando si seleziona o deseleziona un volto. Con altezze diverse (52,7 e 48,7 px nell'app) la
// griglia sotto saltava di 4 px: devono avere la stessa altezza minima e lo stesso padding, e
// l'altezza minima deve superare il contenuto di entrambe (pulsanti fino a 32 px).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const DIR = path.dirname(fileURLToPath(import.meta.url));
// Fine riga normalizzati: in un checkout Windows (core.autocrlf) i CSS hanno CRLF.
const css = fs.readFileSync(path.join(DIR, "people.css"), "utf8").replace(/\r\n/g, "\n");
const rule = (selector: string) =>
  new RegExp(`(^|\\n)${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`).exec(
    css,
  )?.[2] ?? "";
const decl = (body: string, prop: string) =>
  new RegExp(`(^|[;\\s])${prop}\\s*:\\s*([^;]+);`).exec(body)?.[2]?.trim();

describe("intestazione dei volti e barra delle azioni", () => {
  it("hanno la stessa altezza minima e lo stesso padding: la griglia non salta", () => {
    const head = rule(".people-grid-head");
    const bar = rule(".people-face-actions");
    expect(decl(head, "min-height")).toBe("var(--people-grid-head-h)");
    expect(decl(bar, "min-height")).toBe("var(--people-grid-head-h)");
    expect(decl(head, "padding")).toBe(decl(bar, "padding"));

    // Altezza minima > padding verticale + pulsante più alto (32 px) + bordo inferiore (1 px).
    const h = Number(/--people-grid-head-h:\s*(\d+)px/.exec(rule(".people-grid"))?.[1]);
    const padY = Number(/^(\d+)px/.exec(decl(bar, "padding") ?? "")?.[1]);
    expect(h).toBeGreaterThanOrEqual(2 * padY + 32 + 1);
  });
});
