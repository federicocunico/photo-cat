import { describe, expect, it } from "vitest";
import i18n from "../i18n";
import { jobTitle } from "./jobs";

const t = i18n.getFixedT("it");
const paths = (id: number) => (id === 4 ? "/media/foto" : undefined);

describe("titoli delle attività", () => {
  it.each([
    ["import", { sourceId: 4 }, "Importazione di /media/foto"],
    ["import", { sourceId: 99 }, "Importazione"],
    ["export", { destination: "/media/export" }, "Esportazione in /media/export"],
    [
      "sequence_export",
      { options: { destination: "/media/seq" } },
      "Esportazione della sequenza in /media/seq",
    ],
    ["bulk", { ids: [1, 2, 3] }, "Modifica di 3 foto"],
    ["analysis", {}, "Analisi AI"],
    ["models", {}, "Verifica dei modelli AI"],
    ["boh", {}, "Attività"],
  ])("%s %j", (kind, params, title) => {
    expect(jobTitle(t, kind, params, paths)).toBe(title);
  });
});
