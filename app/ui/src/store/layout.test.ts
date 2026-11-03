import { beforeEach, describe, expect, it } from "vitest";
import { destinationFor, migrateLayoutState, useLayout } from "./layout";

beforeEach(() => {
  localStorage.clear();
  useLayout.setState({
    module: "library",
    settingsSection: "general",
    diagFilter: {},
    peopleTarget: null,
  });
});

describe("migrazione delle preferenze del layout", () => {
  it.each([
    ["identities", { module: "people", peopleSection: "people" }],
    ["models", { module: "settings", settingsSection: "models" }],
    ["diagnostics", { module: "settings", settingsSection: "diagnostics" }],
    ["library", { module: "library", settingsSection: "general", peopleSection: "review" }],
  ])("modulo salvato %s", (module, expected) => {
    expect(migrateLayoutState({ module, leftOpen: false })).toMatchObject({
      ...expected,
      leftOpen: false,
    });
  });

  it("all'avvio riapre Impostazioni › Diagnostica da preferenze della versione 1", async () => {
    localStorage.setItem(
      "photo-cat.layout",
      JSON.stringify({ state: { module: "diagnostics", rightOpen: false }, version: 1 }),
    );
    await useLayout.persist.rehydrate();
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
      rightOpen: false,
    });
  });
});

describe("openModule", () => {
  it("apre la Diagnostica come sezione di Impostazioni con il filtro", () => {
    useLayout.getState().openModule({
      module: "settings",
      section: "diagnostics",
      target: { level: "ERROR" },
    });
    expect(useLayout.getState()).toMatchObject({
      module: "settings",
      settingsSection: "diagnostics",
      diagFilter: { level: "ERROR" },
    });
  });

  it("apre Persone su una persona e un modulo semplice senza toccare le sezioni", () => {
    useLayout.getState().openModule({ module: "people", target: { person: 7 } });
    expect(useLayout.getState()).toMatchObject({ module: "people", peopleTarget: { person: 7 } });
    useLayout.getState().openModule({ module: "jobs" });
    expect(useLayout.getState()).toMatchObject({ module: "jobs", settingsSection: "general" });
  });

  it("i vecchi nomi dei moduli diventano destinazioni", () => {
    expect(destinationFor("models")).toEqual({ module: "settings", section: "models" });
    expect(destinationFor("jobs")).toEqual({ module: "jobs" });
    expect(destinationFor("boh")).toBeNull();
  });
});
