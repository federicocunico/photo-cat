// Ogni chiave letterale usata con t("...") deve esistere in italiano e in inglese.
import { describe, expect, it } from "vitest";
import i18n from "./index";

const files = import.meta.glob(["../**/*.{ts,tsx}", "!../**/*.test.{ts,tsx}"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const used = new Set<string>();
for (const text of Object.values(files)) {
  for (const m of text.matchAll(/\bt\(\s*["']([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)+)["']/g))
    used.add(m[1]);
}

// Chiavi composte a runtime (t(`prefisso.${valore}`)): valori elencati qui.
const dynamic: Record<string, string[]> = {
  "onboarding.steps": ["0", "1", "2", "3"],
  "organize.scopes": ["library", "catalog", "selection", "filter"],
  "organize.methods": ["copy", "hardlink", "symlink"],
  "organize.reasons": [
    "symlink_privilege",
    "symlink_denied",
    "cross_volume",
    "hardlink_unsupported",
  ],
  "organize.levels": [
    "place",
    "year",
    "month",
    "person",
    "subject",
    "location",
    "environment",
    "tag",
    "catalog",
  ],
  // Deve coincidere con pc_organize::export::FIELDS.
  "organize.fieldNames": [
    "path",
    "name",
    "format",
    "width",
    "height",
    "dpi_x",
    "dpi_y",
    "bit_depth",
    "size",
    "captured_at",
    "mtime",
    "location",
    "location_source",
    "people_present",
    "people_count",
    "people",
    "subject",
    "location_type",
    "environment",
    "tags",
    "catalogs",
    "labels",
  ],
};
for (const [prefix, values] of Object.entries(dynamic))
  for (const v of values) used.add(`${prefix}.${v}`);

describe("risorse di traduzione", () => {
  it("trova le chiavi usate nel codice", () => expect(used.size).toBeGreaterThan(100));
  for (const lng of ["it", "en"]) {
    it(`ogni chiave usata esiste in ${lng}`, () => {
      const missing = [...used].filter(
        (k) =>
          i18n.getResource(lng, "translation", k) === undefined &&
          i18n.getResource(lng, "translation", `${k}_other`) === undefined,
      );
      expect(missing).toEqual([]);
    });
  }
});
