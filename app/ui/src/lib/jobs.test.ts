import { describe, expect, it } from "vitest";
import i18n from "../i18n";
import { FILE_WRITING_KINDS, jobDoneMessage, jobTitle } from "./jobs";

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
    [
      "raw_review_export",
      { options: { destination: "/media/dolomiti" } },
      "Esportazione in un catalogo in /media/dolomiti",
    ],
    [
      "raw_review_develop",
      { outputDir: "/media/foto/FujiView varianti" },
      "Sviluppo dalla fotocamera in /media/foto/FujiView varianti",
    ],
    ["raw_review_develop", {}, "Sviluppo dalla fotocamera"],
    ["bulk", { ids: [1, 2, 3] }, "Modifica di 3 foto"],
    ["analysis", {}, "Analisi AI"],
    ["models", {}, "Verifica dei modelli AI"],
    ["boh", {}, "Attività"],
  ])("%s %j", (kind, params, title) => {
    expect(jobTitle(t, kind, params, paths)).toBe(title);
  });
});

describe("avviso di fine attività (E6)", () => {
  const phase = (done: number) => ({ done }) as never;
  it("dice che cosa è stato fatto, su che cosa e quante foto", () => {
    const source = (id: number) => (id === 4 ? "D:\\Foto\\Halloween" : undefined);
    expect(
      jobDoneMessage(
        t,
        { kind: "import", phases: [phase(54), phase(54)] },
        { sourceId: 4 },
        source,
      ),
    ).toBe("Importazione di Halloween completata: 54 foto");
    expect(
      jobDoneMessage(
        t,
        { kind: "export", phases: [phase(1)] },
        { destination: "/media/export/Gita" },
      ),
    ).toBe("Esportazione in Gita completata: 1 foto");
  });
  it("concorda il participio e non conta le foto delle attività di servizio", () => {
    expect(jobDoneMessage(t, { kind: "retune", phases: [phase(300)] }, null)).toBe(
      "Ricalcolo delle categorie completato",
    );
    expect(jobDoneMessage(t, { kind: "models", phases: [] }, null)).toBe(
      "Verifica dei modelli AI completata",
    );
  });
});

describe("attività che scrivono file", () => {
  it("lo sviluppo dalla fotocamera chiede conferma prima di essere interrotto", () => {
    expect(FILE_WRITING_KINDS.has("raw_review_develop")).toBe(true);
    expect(FILE_WRITING_KINDS.has("analysis")).toBe(false);
  });
});
