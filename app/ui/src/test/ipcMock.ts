// Mock del modulo lib/ipc: eventi pilotabili dai test e comandi spia.
// Uso: vi.mock("../lib/ipc", () => ipcMockModule()) con percorso relativo al test.

import { vi } from "vitest";
import type { ErrorRecord, JobSnapshot, LogLine, ModelStatus, ResourceSnapshot } from "../bindings";

type Cb<T> = (p: T) => void;

export const emitters = {
  job: [] as Cb<JobSnapshot>[],
  resources: [] as Cb<ResourceSnapshot>[],
  log: [] as Cb<LogLine[]>[],
  error: [] as Cb<ErrorRecord>[],
  models: [] as Cb<ModelStatus[]>[],
};

export function emit<K extends keyof typeof emitters>(
  kind: K,
  payload: Parameters<(typeof emitters)[K][number]>[0],
) {
  for (const cb of emitters[kind] as Cb<typeof payload>[]) cb(payload);
}

export const commandsMock = {
  appInfo: vi.fn(),
  dismissCrashNotice: vi.fn(),
  getConfig: vi.fn(),
  getResources: vi.fn(),
  listJobs: vi.fn(),
  pauseJob: vi.fn(async () => null),
  resumeJob: vi.fn(async () => null),
  cancelJob: vi.fn(async () => null),
  resumeInterruptedJob: vi.fn(async () => null),
  listJobHistory: vi.fn(async () => []),
  startDemoJob: vi.fn(),
  queryErrors: vi.fn(async () => []),
  logTail: vi.fn(async () => []),
  setLogLevel: vi.fn(async () => null),
  listSources: vi.fn(async () => []),
  addSource: vi.fn(),
  updateSource: vi.fn(),
  removeSource: vi.fn(),
  startImport: vi.fn(),
  libraryPage: vi.fn(),
  libraryCount: vi.fn(),
  libraryFacets: vi.fn(),
  libraryIds: vi.fn(),
  mediaDetail: vi.fn(),
  listCatalogs: vi.fn(async () => []),
  systemInfo: vi.fn(),
  listModels: vi.fn(async () => []),
  verifyModels: vi.fn(),
  downloadModel: vi.fn(),
  selfTestModels: vi.fn(),
  installModelFile: vi.fn(),
  setExecutionProvider: vi.fn(),
  setResourceProfile: vi.fn(),
  reanalyze: vi.fn(async () => 1),
  listPeople: vi.fn(async () => []),
  personFaces: vi.fn(async () => []),
  correctPerson: vi.fn(async () => null),
  purgeFaces: vi.fn(async () => null),
  saveCatalog: vi.fn(async () => 1),
  deleteCatalog: vi.fn(async () => null),
  startBulk: vi.fn(async () => 1),
  undoBulk: vi.fn(async () => null),
  lastBulkUndo: vi.fn(async () => null),
  exportFields: vi.fn(async () => ["path", "tags", "people"]),
  startExport: vi.fn(async () => 1),
  organizePreview: vi.fn(),
  organizePreflight: vi.fn(),
  startOrganize: vi.fn(async () => 1),
  listOrganizations: vi.fn(async () => []),
  undoOrganization: vi.fn(async () => [0, 0]),
  faceCount: vi.fn(async () => 0),
  saveSettings: vi.fn(async () => null),
  getTaxonomy: vi.fn(),
  saveTaxonomy: vi.fn(async () => null),
  onboardingDone: vi.fn(async () => true),
  completeOnboarding: vi.fn(async () => null),
  exportDiagnostics: vi.fn(async () => null),
  openLogs: vi.fn(async () => null),
  retryFailed: vi.fn(async () => []),
};

export function ipcMockModule(tauri = false) {
  const reg =
    <T>(list: Cb<T>[]) =>
    (cb: Cb<T>) => {
      list.push(cb);
      return Promise.resolve(() => {
        const i = list.indexOf(cb);
        if (i >= 0) list.splice(i, 1);
      });
    };
  return {
    commands: commandsMock,
    isTauri: () => tauri,
    EVENTS: {},
    events: {
      onJobProgress: reg(emitters.job),
      onResources: reg(emitters.resources),
      onLogLines: reg(emitters.log),
      onError: reg(emitters.error),
      onModels: reg(emitters.models),
    },
  };
}

export function job(partial: Partial<JobSnapshot> & { id: number }): JobSnapshot {
  return {
    kind: "import",
    state: "running",
    overallPercent: 0,
    etaSecs: null,
    failed: 0,
    error: null,
    phases: [],
    ...partial,
  };
}

export function resources(partial: Partial<ResourceSnapshot["system"]> = {}): ResourceSnapshot {
  return {
    system: {
      totalMb: 16384,
      availableMb: 11162,
      processMb: 820,
      logicalCores: 8,
      physicalCores: 4,
      ...partial,
    },
    governor: {
      budgetMb: 4096,
      capacityMb: 4096,
      inUseMb: 600,
      peakMb: 900,
      residents: [["clip", 600]],
      aiPaused: false,
    },
    state: "normal",
  };
}
