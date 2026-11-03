// Ogni chiave letterale usata con t("...") deve esistere in italiano e in inglese.
import { describe, expect, it } from "vitest";
import { features } from "./features";
import i18n from "./index";
import { modules } from "./modules";

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
  "nav.groups": ["browse", "organize", "transfer"],
  "shortcuts.items": [
    "palette",
    "undo",
    "sheet",
    "modules",
    "settings",
    "nav",
    "inspector",
    "panels",
    "move",
    "page",
    "extend",
    "toggle",
    "all",
    "open",
    "deselect",
    "tags",
    "untag",
    "exclude",
    "menu",
    "browse",
    "info",
    "close",
    "session",
    "groups",
    "face",
    "whoIs",
    "moveTo",
    "notSame",
    "merge",
    "ignore",
  ],
  "commands.groups": [
    "general",
    "navigation",
    "library",
    "selection",
    "viewer",
    "sequence",
    "people",
  ],
  "shell.ramState": ["pressure", "critical"],
  "sequence.keys": ["assign", "remove", "session", "fullscreen"],
  "shell.models": ["ready", "missing", "update", "error"],
  "jobs.kinds": [
    "other",
    ...["import", "relink", "export", "organize", "sequence_export", "bulk", "analysis"].flatMap(
      (k) => [k, `${k}_of`],
    ),
    ...["models", "similar", "face_quality", "retune", "time_backfill"],
  ],
  "tags.colors": ["blue", "teal", "yellow", "coral", "purple", "gray"],
  "seqExport.tokens": [
    "seq",
    "seq4",
    "date",
    "time",
    "timeFull",
    "datetime",
    "tag",
    "device",
    "people",
    "place",
    "orig",
  ],
  "jobs.phases": ["orari", "preparazione", "codifica", "per_tag", "manifest", "archivi"],
  // Diagnostica: livelli degli errori e del log (errors.ts).
  "diag.levels": ["error", "warn"],
  "diag.levelFilter": ["error", "warn"],
  "diag.logLevels": ["error", "warn", "info", "debug", "trace"],
  "onboarding.steps": ["0", "1", "2"],
  "guide.steps": ["import", "models", "person", "catalog", "tags"],
  "guide.tips": ["people", "catalogs", "tags", "sequence", "moved"],
  "guide.sections": [
    "library.title",
    "library.body",
    "people.title",
    "people.body",
    "catalogs.title",
    "catalogs.body",
    "tags.title",
    "tags.body",
    "sequence.title",
    "sequence.body",
    "import.title",
    "import.body",
    "settings.title",
    "settings.body",
  ],
  "models.profiles": ["light", "balanced", "performance"],
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
  // GroupList: t(`people.sort.${s}`) con s ∈ GroupSort.
  "people.sort": ["photos_desc", "name_asc", "faces_desc"],
  // ModuleNav: t(`modules.${id}`) — moduli attivi in registry.ts (niente identities).
  modules: [
    "library",
    "sequence",
    "catalogs",
    "tags",
    "people",
    "import",
    "jobs",
    "models",
    "diagnostics",
    "organize",
    "settings",
    "navLabel",
  ],
};
for (const [prefix, values] of Object.entries(dynamic))
  for (const v of values) used.add(`${prefix}.${v}`);

function leafKeys(obj: unknown, prefix: string, out: string[] = []): string[] {
  if (obj && typeof obj === "object") {
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      const key = `${prefix}.${k}`;
      if (v && typeof v === "object") leafKeys(v, key, out);
      else out.push(key);
    }
  }
  return out;
}

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

  it("people.* e identities.* hanno le stesse chiavi in it e en", () => {
    const peopleIt = leafKeys(features.it.people, "people").sort();
    const peopleEn = leafKeys(features.en.people, "people").sort();
    expect(peopleEn).toEqual(peopleIt);

    const idIt = leafKeys(modules.it.identities, "identities").sort();
    const idEn = leafKeys(modules.en.identities, "identities").sort();
    expect(idEn).toEqual(idIt);
  });
});
