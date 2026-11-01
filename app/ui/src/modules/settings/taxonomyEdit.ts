// Modifica della tassonomia in Impostazioni: id dai nomi (stesse regole del
// backend, `pc-ai/src/taxonomy.rs`), pulizia prima del salvataggio e confronto
// della parte che determina l'analisi automatica.
import type { Category, Group, Taxonomy } from "../../lib/ipc";

const fold = (s: string) => s.trim().toLowerCase().normalize("NFD").replace(/\p{M}/gu, "");

function slug(name: string, allowed: RegExp) {
  let out = "";
  for (const ch of fold(name)) {
    const c = allowed.test(ch) ? ch : "_";
    if (c === "_" && (out === "" || out.endsWith("_"))) continue;
    out += c;
  }
  return out.replace(/_+$/, "");
}

export function categoryId(group: Group, name: string) {
  const base = slug(name, /[a-z0-9]/) || "voce";
  const taken = (id: string) => group.categories.some((c) => c.id === id);
  if (!taken(base)) return base;
  for (let n = 2; ; n++) if (!taken(`${base}_${n}`)) return `${base}_${n}`;
}

const letters = (n: number) => {
  let s = "";
  for (;;) {
    s = String.fromCharCode(97 + (n % 26)) + s;
    if (n < 26) return s;
    n = Math.floor(n / 26) - 1;
  }
};

/** Id di gruppo: solo `[a-z_]`, come richiede la scrittura delle etichette. */
export function groupId(tax: Taxonomy, name: string) {
  const base = slug(name, /[a-z]/) || "gruppo";
  const taken = (id: string) => tax.groups.some((g) => g.id === id);
  if (!taken(base)) return base;
  for (let n = 1; ; n++) if (!taken(`${base}_${letters(n)}`)) return `${base}_${letters(n)}`;
}

export const isManual = (c: Category) => c.prompts.every((p) => !p.trim());

/** Prompt senza righe vuote: una voce senza descrizione è manuale. */
export const cleanTaxonomy = (tax: Taxonomy): Taxonomy => ({
  ...tax,
  groups: tax.groups.map((g) => ({
    ...g,
    categories: g.categories.map((c) => ({
      ...c,
      prompts: c.prompts.map((p) => p.trim()).filter(Boolean),
    })),
  })),
});

/** Parte della tassonomia usata dall'analisi automatica (come `analysis_signature`). */
function signature(tax: Taxonomy) {
  return JSON.stringify(
    cleanTaxonomy(tax)
      .groups.map((g) => ({
        id: g.id,
        threshold: g.threshold,
        cats: g.categories
          .filter((c) => c.enabled && c.prompts.length > 0)
          .map((c) => ({
            id: c.id,
            prompts: [...c.prompts].sort(),
            threshold: c.threshold ?? null,
          }))
          .sort((a, b) => a.id.localeCompare(b.id)),
      }))
      .filter((g) => g.cats.length > 0)
      .sort((a, b) => a.id.localeCompare(b.id)),
  );
}

export const changesAnalysis = (a: Taxonomy, b: Taxonomy) => signature(a) !== signature(b);
