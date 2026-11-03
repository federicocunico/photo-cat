// Ogni chiave di traduzione è usata da qualche parte (letterale o con un prefisso
// dinamico t(`prefisso.${valore}`)), e nessun modulo usa window.confirm o window.alert:
// le conferme passano da ConfirmDialog o dallo store `confirm` (ui-ux-overhaul 8.1).
import { expect, it } from "vitest";
import i18n from "./index";

const files = import.meta.glob(
  ["../**/*.{ts,tsx}", "!../**/*.test.{ts,tsx}", "!../i18n/*.ts", "!../test/**"],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;
const text = Object.values(files).join("\n");

function flat(o: Record<string, unknown>, p = "", out: string[] = []) {
  for (const [k, v] of Object.entries(o)) {
    const key = p ? `${p}.${k}` : k;
    if (typeof v === "string") out.push(key.replace(/_(one|other|zero|few|many)$/, ""));
    else flat(v as Record<string, unknown>, key, out);
  }
  return out;
}

// Termini del glossario richiesti da glossary.test.ts anche se oggi nessun modulo li usa.
const GLOSSARY = ["common.stop", "common.retry"];

it("nessuna chiave di traduzione inutilizzata", () => {
  const keys = [...new Set(flat(i18n.getResourceBundle("it", "translation")))];
  const unused = keys.filter((k) => {
    if (GLOSSARY.includes(k)) return false;
    if (text.includes(`"${k}"`) || text.includes(`'${k}'`)) return false;
    const parts = k.split(".");
    for (let i = parts.length - 1; i > 0; i--)
      if (text.includes("`" + parts.slice(0, i).join(".") + ".${")) return false;
    return true;
  });
  expect(unused).toEqual([]);
});

it("nessun window.confirm o window.alert", () => {
  const found = Object.entries(files)
    .filter(([, src]) => /window\.(confirm|alert)\s*\(/.test(src))
    .map(([f]) => f);
  expect(found).toEqual([]);
});
