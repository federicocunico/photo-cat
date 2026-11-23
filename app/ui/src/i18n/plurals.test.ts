// Ogni testo con un conteggio ({{count}}) ha le forme singolare e plurale (F7): i18next sceglie
// `_one` o `_other` solo se le due chiavi esistono.
import { describe, expect, it } from "vitest";
import i18n from "./index";

type Tree = { [key: string]: string | Tree };

function flatten(tree: Tree, prefix = "", out: Array<[string, string]> = []) {
  for (const [k, v] of Object.entries(tree)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out.push([key, v]);
    else flatten(v, key, out);
  }
  return out;
}

const PLURAL = /_(zero|one|two|few|many|other)$/;

describe.each(["it", "en"])("plurali in %s", (lng) => {
  const entries = flatten(i18n.getResourceBundle(lng, "translation") as Tree);
  const keys = new Set(entries.map(([k]) => k));

  it("ogni testo con {{count}} è una forma plurale", () => {
    const bare = entries.filter(([k, v]) => v.includes("{{count}}") && !PLURAL.test(k));
    expect(bare.map(([k]) => k)).toEqual([]);
  });

  it("ogni forma _one ha la sua _other e viceversa", () => {
    const orphans = [...keys].filter((k) => {
      if (k.endsWith("_one")) return !keys.has(k.replace(/_one$/, "_other"));
      if (k.endsWith("_other")) return !keys.has(k.replace(/_other$/, "_one"));
      return false;
    });
    expect(orphans).toEqual([]);
  });
});
