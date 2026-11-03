// Termini vietati per lingua (i18n/GLOSSARY.md) e chiavi comuni obbligatorie.
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

const JARGON = /\b(eq|neq|gte|lte|people_count|self-test|sha-256|glob|ssim|embeddings?)\b/i;

const BANNED: Record<"it" | "en", Record<string, RegExp>> = {
  it: {
    job: /\bjobs?\b/i,
    sorgente: /\bsorgent[ei]\b/i,
    annullaUltimaAzione: /annulla ultima azione/i,
    gergo: JARGON,
  },
  en: {
    job: /\bjobs?\b/i,
    undoLastAction: /undo last action/i,
    gergo: JARGON,
  },
};

/**
 * Stringhe non ancora allineate al glossario, con il task che le riscrive.
 * L'elenco può solo accorciarsi: il task 8.1 lo svuota.
 */
const PENDING: Record<string, string[]> = {
  "it:job": [],
  "en:job": [],
  // 7.1 (Importa: "cartella" al posto di "sorgente").
  "it:sorgente": [
    "import.title",
    "import.sources",
    "library.empty",
    "hints.not_found",
    "organize.reasons.cross_volume",
    "sequence.otherSources",
    "sequence.sources",
    "seqExport.namesHelp",
    "sources.relinkDialog.intro_one",
    "sources.relinkDialog.intro_other",
    "sources.relinkDialog.outside",
    "sources.relinkDialog.addSource",
    "sources.relinkDialog.moveSource",
    "sources.relinkDialog.sourceAdded",
    "sources.relinkDialog.sourceMoved",
    "sources.missingDialog.body_one",
    "sources.missingDialog.body_other",
  ],
  // 3.2/3.3 (SelectionBar e store undo).
  "it:annullaUltimaAzione": ["bulk.undo"],
  "en:undoLastAction": ["bulk.undo"],
  // 6.3 (Sensibilità, "Elimina tutti i dati dei volti"), 6.1 (Modelli AI), 7.5 (export di Sequenza).
  "it:gergo": [
    "models.selfTests",
    "settings.purgeConfirm",
    "seqExport.verify",
    "seqExport.outManifest",
  ],
  "en:gergo": [
    "errors.self_test_failed",
    "settings.purgeConfirm",
    "seqExport.verify",
    "seqExport.outManifest",
  ],
};

describe("glossario", () => {
  for (const lng of ["it", "en"] as const) {
    const strings = flatten(i18n.getResourceBundle(lng, "translation") as Tree);
    for (const [term, re] of Object.entries(BANNED[lng])) {
      it(`${lng}: nessun "${term}" fuori dalle eccezioni`, () => {
        const pending = PENDING[`${lng}:${term}`] ?? [];
        const hits = strings.filter(([, v]) => re.test(v)).map(([k]) => k);
        expect(hits.filter((k) => !pending.includes(k))).toEqual([]);
        // Una stringa corretta esce dall'elenco delle eccezioni.
        expect(pending.filter((k) => !hits.includes(k))).toEqual([]);
      });
    }
  }

  it("chiavi comuni del glossario", () => {
    const expected = {
      it: {
        cancel: "Annulla",
        save: "Salva",
        close: "Chiudi",
        stop: "Interrompi",
        undo: "Annulla operazione",
        retry: "Riprova",
        details: "Dettagli",
      },
      en: {
        cancel: "Cancel",
        save: "Save",
        close: "Close",
        stop: "Stop",
        undo: "Undo",
        retry: "Retry",
        details: "Details",
      },
    };
    for (const lng of ["it", "en"] as const) {
      const t = i18n.getFixedT(lng);
      for (const [key, text] of Object.entries(expected[lng])) {
        expect(t(`common.${key}`)).toBe(text);
      }
    }
  });
});
