import { afterEach, describe, expect, it } from "vitest";
import i18n from "../../i18n";
import type { MetaValueKey } from "../../lib/ipc";
import { metaLabel, metaSectionTitle, metaValue, recipeSummary } from "./metaText";

afterEach(async () => {
  await i18n.changeLanguage("it");
});

const t = () => i18n.t.bind(i18n);

describe("testi dei metadati di Varianti RAW", () => {
  it("traduce sezioni, etichette e valori enumerati in italiano e in inglese", async () => {
    const row = { label: "whiteBalance", value: "Luce diurna", valueKey: "wbDaylight" as const };
    expect(metaSectionTitle(t(), { title: "shot" })).toBe("Scatto");
    expect(metaLabel(t(), row)).toBe("Bilanciamento bianco");
    expect(metaValue(t(), row)).toBe("Luce diurna");
    await i18n.changeLanguage("en");
    expect(metaSectionTitle(t(), { title: "shot" })).toBe("Shot");
    expect(metaLabel(t(), row)).toBe("White balance");
    expect(metaValue(t(), row)).toBe("Daylight");
    expect(metaValue(t(), { value: "Seppia", valueKey: "filmSepia" })).toBe("Sepia");
  });

  it("senza chiave o con una chiave sconosciuta mostra il testo del backend", async () => {
    await i18n.changeLanguage("en");
    expect(metaValue(t(), { value: "1/250 s", valueKey: null })).toBe("1/250 s");
    expect(metaValue(t(), { value: "Nuovo valore", valueKey: "nonEsiste" as MetaValueKey })).toBe(
      "Nuovo valore",
    );
    expect(metaLabel(t(), { label: "nuovaEtichetta" })).toBe("nuovaEtichetta");
  });

  it("ricompone la ricetta nella lingua dell'app", async () => {
    const recipe = {
      summary: "Monocromatico · DR200 · Ombra",
      film: "Monocromatico",
      filmKey: "filmMonochrome" as const,
      dynamicRange: "DR200",
      whiteBalance: "Ombra",
      whiteBalanceKey: "wbShade" as const,
    };
    expect(recipeSummary(t(), recipe)).toBe("Monocromatico · DR200 · Ombra");
    await i18n.changeLanguage("en");
    expect(recipeSummary(t(), recipe)).toBe("Monochrome · DR200 · Shade");
    expect(
      recipeSummary(t(), {
        ...recipe,
        film: "Classic Chrome",
        filmKey: null,
        whiteBalance: null,
        whiteBalanceKey: null,
      }),
    ).toBe("Classic Chrome · DR200");
  });
});
