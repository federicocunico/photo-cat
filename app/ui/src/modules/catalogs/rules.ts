// Regole dei cataloghi smart: campi, confronti ammessi e frase di riepilogo.
// I nomi di campo e di confronto sono quelli di `pc-store/src/rules.rs`: qui restano
// interni, l'interfaccia li mostra sempre con le etichette tradotte.
import type { TFunction } from "i18next";
import { fmtInt } from "../../lib/format";

export type Cmp = "eq" | "neq" | "gt" | "gte" | "lt" | "lte" | "contains" | "in";
export type Cond = { field: string; cmp: string; value: string | number | Array<string | number> };
export type RuleNode = { op: "and" | "or"; rules: RuleNode[] } | Cond;

/** Tipo di valore di un campo: decide i confronti e il selettore del valore. */
export type FieldKind =
  | "label"
  | "tag"
  | "person"
  | "place"
  | "format"
  | "text"
  | "status"
  | "year"
  | "count"
  | "pixels"
  | "size";

const KINDS: Record<string, FieldKind> = {
  tag: "tag",
  person: "person",
  place: "place",
  format: "format",
  year: "year",
  people_count: "count",
  width: "pixels",
  height: "pixels",
  size: "size",
  name: "text",
  status: "status",
};

/** Campi offerti nell'editor dopo i gruppi della tassonomia (`label.<gruppo>`). */
export const OTHER_FIELDS = Object.keys(KINDS);

export const kindOf = (field: string): FieldKind | null =>
  field.startsWith("label.") ? "label" : (KINDS[field] ?? null);

const NUMERIC = new Set<FieldKind>(["year", "count", "pixels", "size"]);
export const isNumeric = (kind: FieldKind | null) => kind != null && NUMERIC.has(kind);

/** Confronti ammessi dal backend per ogni tipo di campo. */
export function comparisons(kind: FieldKind | null): Cmp[] {
  if (kind == null) return ["eq", "neq", "gt", "gte", "lt", "lte", "contains", "in"];
  if (isNumeric(kind)) return ["eq", "neq", "gt", "gte", "lt", "lte"];
  if (kind === "place" || kind === "text") return ["eq", "neq", "contains"];
  return ["eq", "neq"];
}

/** 1 MB come lo mostra l'app (`fmtBytes`). */
export const MB = 1024 * 1024;

export function defaultValue(kind: FieldKind | null): string | number {
  switch (kind) {
    case "year":
      return new Date().getFullYear();
    case "count":
      return 1;
    case "pixels":
      return 1000;
    case "size":
      return 10 * MB;
    case "status":
      return "ok";
    default:
      return "";
  }
}

export const newRule = (): RuleNode => ({ field: "label.subject", cmp: "eq", value: "paesaggio" });

/** La condizione con un campo nuovo: confronto e valore tornano validi per quel campo. */
export function withField(rule: Cond, field: string): Cond {
  const kind = kindOf(field);
  const cmps = comparisons(kind);
  const cmp = cmps.includes(rule.cmp as Cmp) ? rule.cmp : "eq";
  // Larghezza → altezza tiene il numero; un altro gruppo di etichette ha altre voci.
  const keep = kind === kindOf(rule.field) && kind !== "label";
  return { field, cmp, value: keep ? rule.value : defaultValue(kind) };
}

type Names = { group: (g: string) => string; label: (g: string, v: string) => string };

/** Nome del campo nella lingua dell'interfaccia (mai l'id interno). */
export function fieldName(t: TFunction, names: Names, field: string): string {
  if (field.startsWith("label.")) return names.group(field.slice(6));
  const kind = kindOf(field);
  if (kind || field === "catalog") return t(`catalogs.fields.${field}`);
  return t("catalogs.fields.other");
}

/** Valore in parole, con l'unità per i numeri. */
export function valueText(t: TFunction, names: Names, field: string, value: Cond["value"]): string {
  if (Array.isArray(value)) return value.map((v) => valueText(t, names, field, v)).join(", ");
  const kind = kindOf(field);
  if (value === "" || value == null) return t("catalogs.summary.missing");
  if (kind === "label") return names.label(field.slice(6), String(value));
  if (kind === "status")
    return value === "ok" || value === "missing" ? t(`catalogs.status.${value}`) : String(value);
  const n = Number(value);
  if (kind === "year") return String(value);
  if (kind === "count") return `${fmtInt(n)} ${t("catalogs.units.people", { count: n })}`;
  if (kind === "pixels") return `${fmtInt(n)} ${t("catalogs.units.px")}`;
  if (kind === "size") return `${fmtInt(Math.round(n / MB))} ${t("catalogs.units.mb")}`;
  if (kind === "format") return String(value).toUpperCase();
  return String(value);
}

const YEAR_KEYS: Record<string, string> = {
  eq: "catalogs.summary.yearEq",
  neq: "catalogs.summary.yearNeq",
  gt: "catalogs.summary.yearGt",
  gte: "catalogs.summary.yearGte",
  lt: "catalogs.summary.yearLt",
  lte: "catalogs.summary.yearLte",
};

function condPhrase(t: TFunction, names: Names, c: Cond): string {
  const kind = kindOf(c.field);
  const value = valueText(t, names, c.field, c.value);
  if (kind === "year" && YEAR_KEYS[c.cmp]) return t(YEAR_KEYS[c.cmp], { value });
  if (kind === "tag" && (c.cmp === "eq" || c.cmp === "neq"))
    return t(c.cmp === "eq" ? "catalogs.summary.tagEq" : "catalogs.summary.tagNeq", { value });
  if (kind === "person" && (c.cmp === "eq" || c.cmp === "neq"))
    return t(c.cmp === "eq" ? "catalogs.summary.personEq" : "catalogs.summary.personNeq", {
      value,
    });
  const field = fieldName(t, names, c.field).toLocaleLowerCase();
  const cmp = (comparisons(null) as string[]).includes(c.cmp) ? c.cmp : "eq";
  return t(`catalogs.summary.${cmp}`, { field, value });
}

function phrase(t: TFunction, names: Names, rule: RuleNode, nested: boolean): string {
  if (!("op" in rule)) return condPhrase(t, names, rule);
  const parts = rule.rules
    .map((r) => phrase(t, names, r, true))
    .join(t(rule.op === "and" ? "catalogs.summary.and" : "catalogs.summary.or"));
  return nested && rule.rules.length > 1 ? t("catalogs.summary.group", { parts }) : parts;
}

/** Riepilogo della regola in una frase: "Foto con tipo soggetto Ritratto, dal 2022 in poi". */
export function summarize(t: TFunction, names: Names, rule: RuleNode): string {
  return t("catalogs.summary.sentence", { parts: phrase(t, names, rule, false) });
}
