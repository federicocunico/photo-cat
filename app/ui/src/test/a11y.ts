// Controllo di accessibilità sul DOM renderizzato dai test delle pagine (ui-foundations,
// "Accessibilità"): bersagli sotto 24 px, controlli annidati, comandi senza nome.
import { cleanup } from "@testing-library/react";
import { expect } from "vitest";

const sheets = import.meta.glob("../**/*.css", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** jsdom calcola lo stile dai fogli presenti nel documento: si caricano quelli dell'app. */
function ensureStyles() {
  if (document.head.querySelector("style[data-a11y]")) return;
  for (const css of Object.values(sheets)) {
    const style = document.createElement("style");
    style.dataset.a11y = "";
    style.textContent = css;
    document.head.appendChild(style);
  }
}

const CONTROL = [
  "button",
  "a[href]",
  "input:not([type=hidden])",
  "select",
  "textarea",
  "summary",
  ...[
    "button",
    "link",
    "checkbox",
    "radio",
    "switch",
    "tab",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "slider",
  ].map((r) => `[role=${r}]`),
].join(",");

const NAMED = "button, a[href], [role=button], [role=link], [role=tab], [role=menuitem]";

/** Il minimo WCAG 2.5.8. */
export const MIN_TARGET = 24;

/**
 * Nome accessibile semplificato: aria-labelledby, aria-label, etichette dei campi,
 * testo visibile ai lettori di schermo (con `alt` delle immagini), poi `title`.
 */
function accessibleName(el: Element): string {
  const ids = el.getAttribute("aria-labelledby");
  if (ids)
    return ids
      .split(/s+/)
      .map((id) => document.getElementById(id)?.textContent ?? "")
      .join(" ")
      .trim();
  const label = el.getAttribute("aria-label")?.trim();
  if (label) return label;
  if (
    el instanceof HTMLInputElement ||
    el instanceof HTMLSelectElement ||
    el instanceof HTMLTextAreaElement
  ) {
    const text = Array.from(el.labels ?? [])
      .map((l) => l.textContent ?? "")
      .join(" ")
      .trim();
    if (text) return text;
  }
  const text = visibleText(el).trim();
  return text || (el.getAttribute("title") ?? "").trim();
}

function visibleText(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) return node.textContent ?? "";
  if (!(node instanceof Element) || node.getAttribute("aria-hidden") === "true") return "";
  if (node instanceof HTMLImageElement) return node.alt;
  return Array.from(node.childNodes).map(visibleText).join("");
}

const px = (v: string) => (v.endsWith("px") ? parseFloat(v) : NaN);

function hidden(el: Element): boolean {
  return el.closest("[hidden], [aria-hidden=true], [inert]") != null;
}

function describe(el: Element): string {
  const tag = el.tagName.toLowerCase();
  const id = el.getAttribute("data-testid");
  if (id) return `${tag}[data-testid=${id}]`;
  const name = accessibleName(el).trim().replace(/\s+/g, " ");
  if (name) return `${tag} "${name.slice(0, 40)}"`;
  const cls = el.getAttribute("class");
  return cls ? `${tag}.${cls.split(/\s+/).join(".")}` : tag;
}

/**
 * Lato del bersaglio noto dallo stile (dimensione o minimo, esplicito o dalle classi).
 * `NaN` se lo stile non lo fissa: jsdom non calcola il layout, quindi non si stima.
 */
function side(style: CSSStyleDeclaration, axis: "width" | "height"): number {
  const values = [style[axis], style[axis === "width" ? "minWidth" : "minHeight"]]
    .map(px)
    .filter((n) => !Number.isNaN(n));
  return values.length ? Math.max(...values) : NaN;
}

/** Un bersaglio inline in un testo, o una casella dentro la sua etichetta, è esente. */
function exempt(el: Element): boolean {
  if (el.tagName === "A" && el.parentElement?.closest("p, li") && !el.className) return true;
  if (el instanceof HTMLInputElement && ["checkbox", "radio"].includes(el.type))
    return el.closest("label") != null;
  return false;
}

export type A11yIssue = string;

/** Problemi trovati sotto `root`, come testo leggibile ("size: button "Chiudi" 20×20"). */
export function auditA11y(root: ParentNode = document.body): A11yIssue[] {
  ensureStyles();
  const issues: A11yIssue[] = [];
  const controls = Array.from(root.querySelectorAll(CONTROL)).filter((el) => !hidden(el));
  for (const el of controls) {
    const nested = Array.from(el.querySelectorAll(CONTROL)).find((c) => !hidden(c));
    if (nested) issues.push(`nested: ${describe(nested)} in ${describe(el)}`);

    if (el.matches(NAMED) && !accessibleName(el).trim()) issues.push(`unnamed: ${describe(el)}`);

    if (!exempt(el)) {
      const style = getComputedStyle(el);
      const w = side(style, "width");
      const h = side(style, "height");
      if (w < MIN_TARGET || h < MIN_TARGET)
        issues.push(
          `size: ${describe(el)} ${Number.isNaN(w) ? "?" : w}×${Number.isNaN(h) ? "?" : h}`,
        );
    }
  }
  return Array.from(new Set(issues));
}

/**
 * Fallisce se `root` ha problemi diversi da quelli noti. `known` contiene prefissi dei
 * problemi ancora aperti, ognuno legato al task che lo risolve (`a11yKnown.ts`).
 */
export function expectAccessible(root: ParentNode = document.body, known: readonly string[] = []) {
  const issues = auditA11y(root).filter((i) => !known.some((k) => i.startsWith(k)));
  expect(issues).toEqual([]);
}

/**
 * Da usare al posto di `afterEach(cleanup)` nei test delle pagine: controlla il DOM
 * renderizzato da ogni test, poi lo smonta comunque.
 */
export function checkedCleanup(known: readonly string[] = []) {
  return () => {
    try {
      expectAccessible(document.body, known);
    } finally {
      cleanup();
    }
  };
}
