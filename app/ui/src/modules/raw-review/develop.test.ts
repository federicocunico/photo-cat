import { describe, expect, it } from "vitest";
import type {
  CameraDiagnostics,
  CameraPreset,
  CameraStatus,
  DiagnosticCheck,
  VariantPlan,
} from "../../lib/ipc";
import {
  appendLog,
  effectiveSelection,
  firstProblem,
  formatClock,
  headline,
  logToText,
  nextPollDelay,
  startBlock,
  togglePreset,
} from "./develop";

const preset = (id: string): CameraPreset => ({
  id,
  bank: null,
  name: id,
  folderName: id,
  filmSimulation: null,
  summary: "",
  unappliedProperties: [],
});
const status = (over: Partial<CameraStatus> = {}): CameraStatus => ({
  available: true,
  state: "ready",
  model: "X-T5",
  firmware: "4.31",
  message: null,
  simulated: false,
  ...over,
});
const check = (id: string, s: DiagnosticCheck["status"]): DiagnosticCheck => ({
  id,
  label: id,
  status: s,
  detail: null,
  hint: null,
});
const diag = (over: Partial<CameraDiagnostics> = {}): CameraDiagnostics => ({
  checks: [],
  model: null,
  firmware: null,
  simulated: false,
  checkedAt: 0,
  ready: true,
  unresponsive: false,
  running: false,
  ...over,
});
const plan = (over: Partial<VariantPlan> = {}): VariantPlan => ({
  sourceRoot: "D:/Foto",
  outputDir: "D:/Foto/FujiView varianti",
  rafCount: 2,
  presets: [],
  totalConversions: 4,
  pendingConversions: 4,
  alreadyDone: 0,
  estimatedBytes: 1,
  freeBytes: 10,
  safetyMarginBytes: 1,
  enoughSpace: true,
  estimatedSeconds: 10,
  warnings: [],
  ...over,
});
const base = {
  status: status(),
  diagnostics: diag(),
  checking: false,
  presetCount: 2,
  shotCount: null,
  plan: plan(),
  planning: false,
  planFailed: false,
};

describe("scelta dei preset", () => {
  it("aggiunge e toglie un preset", () => {
    expect(togglePreset(["C1"], "C2")).toEqual(["C1", "C2"]);
    expect(togglePreset(["C1", "C2"], "C1")).toEqual(["C2"]);
  });

  it("parte dalla scelta ricordata e ignora i preset che non esistono più", () => {
    const all = ["asshot", "C1", "C2", "C3"].map(preset);
    expect(effectiveSelection(all, ["C1", "C3", "C9"], null)).toEqual(["C1", "C3"]);
  });

  it("senza scelta ricordata valida parte da tutti, ma una scelta vuota dell'utente resta vuota", () => {
    const all = ["asshot", "C1"].map(preset);
    expect(effectiveSelection(all, [], null)).toEqual(["asshot", "C1"]);
    expect(effectiveSelection(all, ["C1"], [])).toEqual([]);
  });
});

describe("avvio dello sviluppo", () => {
  it("parte solo con fotocamera pronta, controlli superati e piano valido", () => {
    expect(startBlock(base)).toBeNull();
  });

  it("indica il motivo per ogni ostacolo, nell'ordine", () => {
    expect(startBlock({ ...base, status: undefined })).toBe("camera_unknown");
    expect(startBlock({ ...base, status: status({ available: false }) })).toBe("unavailable");
    expect(startBlock({ ...base, status: status({ state: "busy" }) })).toBe("camera_busy");
    expect(startBlock({ ...base, checking: true })).toBe("checking");
    expect(startBlock({ ...base, diagnostics: undefined })).toBe("checking");
    expect(startBlock({ ...base, diagnostics: diag({ ready: false }) })).toBe("checks");
    expect(startBlock({ ...base, presetCount: 0 })).toBe("noPresets");
    expect(startBlock({ ...base, shotCount: 0 })).toBe("noShots");
    expect(startBlock({ ...base, planning: true })).toBe("planning");
    expect(startBlock({ ...base, planFailed: true })).toBe("planError");
    expect(startBlock({ ...base, plan: plan({ enoughSpace: false }) })).toBe("space");
    expect(startBlock({ ...base, plan: plan({ pendingConversions: 0 }) })).toBe("nothingToDo");
  });
});

describe("verifica della fotocamera", () => {
  it("il primo problema è un errore prima di un avviso", () => {
    const d = diag({
      ready: false,
      checks: [check("a", "ok"), check("b", "warning"), check("c", "error")],
    });
    expect(firstProblem(d)?.id).toBe("c");
    expect(firstProblem(diag({ checks: [check("a", "ok")] }))).toBeUndefined();
  });

  it("l'intestazione segue lo stato", () => {
    expect(headline(status(), undefined, true)).toBe("checking");
    expect(headline(status({ state: "unresponsive" }), diag(), false)).toBe("waiting");
    expect(headline(status(), diag({ unresponsive: true, ready: false }), false)).toBe("frozen");
    expect(headline(status({ state: "disconnected" }), undefined, false)).toBe("none");
    expect(headline(status(), diag(), false)).toBe("ready");
    expect(headline(status(), diag({ ready: false }), false)).toBe("problems");
  });

  it("controlla più di rado una fotocamera che non risponde e mai a finestra nascosta", () => {
    expect(nextPollDelay("ready", false)).toBe(5000);
    expect(nextPollDelay("unresponsive", false)).toBe(15000);
    expect(nextPollDelay("ready", true)).toBeNull();
  });
});

describe("registro della fotocamera", () => {
  it("tiene solo le ultime righe e si copia come testo", () => {
    let lines: ReturnType<typeof appendLog> = [];
    for (let i = 0; i < 5; i++)
      lines = appendLog(lines, { at: 0, level: "info", message: `m${i}` }, 3);
    expect(lines.map((l) => l.message)).toEqual(["m2", "m3", "m4"]);
    expect(logToText(lines.slice(0, 1))).toBe(`${formatClock(0)} [info] m2`);
  });
});
