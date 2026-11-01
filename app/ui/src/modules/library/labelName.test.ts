import { afterEach, describe, expect, it } from "vitest";
import i18n from "../../i18n";
import type { Taxonomy } from "../../lib/ipc";
import { groupNameOf, labelNameOf } from "./labelName";

const tax = (fiori: string): Taxonomy => ({
  version: 3,
  groups: [
    {
      id: "subject",
      label: "Tipo soggetto",
      threshold: 0.25,
      categories: [
        { id: "fiori", label: fiori, enabled: true, prompts: ["flowers"] },
        { id: "barche", label: "Barche", enabled: true, prompts: [] },
      ],
    },
    {
      id: "evento",
      label: "Evento",
      threshold: 0.25,
      categories: [{ id: "compleanno", label: "Compleanno", enabled: true, prompts: [] }],
    },
  ],
});

afterEach(() => i18n.changeLanguage("it"));

describe("nome delle voci", () => {
  it("voce creata dall'utente: il suo nome", () => {
    expect(labelNameOf(tax("Fiori"), i18n.t, "subject", "barche")).toBe("Barche");
    expect(labelNameOf(tax("Fiori"), i18n.t, "evento", "compleanno")).toBe("Compleanno");
    expect(groupNameOf(tax("Fiori"), i18n.t, "evento")).toBe("Evento");
  });

  it("voce predefinita: la traduzione", async () => {
    await i18n.changeLanguage("en");
    expect(labelNameOf(tax("Fiori"), i18n.t, "subject", "fiori")).toBe("Flowers");
    expect(groupNameOf(tax("Fiori"), i18n.t, "subject")).toBe("Subject type");
    await i18n.changeLanguage("it");
    expect(labelNameOf(tax("Fiori"), i18n.t, "subject", "fiori")).toBe("Fiori");
  });

  it("voce predefinita rinominata: il nuovo nome", async () => {
    await i18n.changeLanguage("en");
    expect(labelNameOf(tax("Fiori e piante"), i18n.t, "subject", "fiori")).toBe("Fiori e piante");
  });

  it("senza tassonomia o voce sconosciuta: traduzione o id leggibile", () => {
    expect(labelNameOf(undefined, i18n.t, "subject", "opera_arte")).toBe("Opera d'arte");
    expect(labelNameOf(undefined, i18n.t, "subject", "gita_scolastica")).toBe("gita scolastica");
  });
});
