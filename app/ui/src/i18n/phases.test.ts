// Ogni fase emessa dal backend ha un nome in parole in italiano e in inglese (F4): l'elenco è
// generato dal test Rust `app/src-tauri/tests/phases.rs`.
import { describe, expect, it } from "vitest";
import phases from "../test/fixtures/phases.json";
import i18n from "./index";

describe("nomi delle fasi", () => {
  it.each(["it", "en"])("ogni fase ha una traduzione in %s", (lng) => {
    const missing = (phases as string[]).filter(
      (name) => !i18n.exists(`jobs.phases.${name}`, { lng }),
    );
    expect(missing).toEqual([]);
  });

  it("l'elenco contiene le fasi dell'analisi AI dell'import", () => {
    expect(phases).toEqual(expect.arrayContaining(["geo", "clip", "persons", "faces"]));
  });
});
