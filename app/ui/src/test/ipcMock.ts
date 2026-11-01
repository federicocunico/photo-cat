// Mock del modulo lib/ipc: eventi pilotabili dai test e comandi spia.
// Uso: vi.mock("../lib/ipc", () => ipcMockModule()) con percorso relativo al test.

import { vi } from "vitest";
import type {
  ErrorRecord,
  JobSnapshot,
  LogLine,
  ModelStatus,
  RenameOutcome,
  IdentitiesView,
  RefFaceView,
  AddReferenceOutcome,
  IdentitySuggestion,
  SequenceScope,
  SequenceOverview,
  SequenceCursor,
  SequencePage,
  ClockPreview,
  Alignment,
  ExportDefaults,
  ExportPreflight,
  NamePreviewResult,
  SequenceExportOptions,
  ResourceSnapshot,
  TagOverview,
  FolderNode,
  PlaceSuggestion,
  OpenTarget,
  OpenOutcome,
  AiThresholds,
  Taxonomy,
  ThresholdImpact,
  UpdateSummary,
  RelinkSummary,
} from "../bindings";

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
  sourceRemovalSummary: vi.fn(),
  relinkMissing: vi.fn(),
  removeMissing: vi.fn(),
  moveSource: vi.fn(),
  missingFolder: vi.fn(async () => null as string | null),
  importUpdates: vi.fn(async () => [] as UpdateSummary[]),
  lastRelink: vi.fn(async () => null as RelinkSummary | null),
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
  tagOverview: vi.fn(async (): Promise<TagOverview> => ({
    tags: [],
    sets: [],
    activeSet: null,
    palette: [],
  })),
  renameTag: vi.fn(async (): Promise<RenameOutcome> => ({ outcome: "renamed" })),
  mergeTag: vi.fn(async () => ({ changed: 0, requested: 0 })),
  deleteTag: vi.fn(async () => ({ changed: 0, requested: 0 })),
  setTagColor: vi.fn(async () => null),
  saveTagSet: vi.fn(async () => 1),
  duplicateTagSet: vi.fn(async () => 2),
  deleteTagSet: vi.fn(async () => null),
  reorderTagSets: vi.fn(async () => null),
  addTagToSet: vi.fn(async () => 1),
  removeTagFromSet: vi.fn(async () => null),
  setActiveTagSet: vi.fn(async () => null),
  setTagSetExportKeywords: vi.fn(async () => null),
  setTagSetExclusive: vi.fn(async (): Promise<number[]> => []),
  tagSetConflicts: vi.fn(async (): Promise<number[]> => []),
  listIdentities: vi.fn<(q: string | null) => Promise<IdentitiesView>>(async () => ({
    identities: [],
    groups: [],
    storePath: "",
    defaultThreshold: 0.363,
  })),
  saveIdentity: vi.fn(async () => "uid-1"),
  deleteIdentity: vi.fn(async () => null),
  mergeIdentities: vi.fn(async () => null),
  identityFaces: vi.fn(async (): Promise<RefFaceView[]> => []),
  identityFaceImage: vi.fn(async () => "data:image/png;base64,"),
  removeIdentityFace: vi.fn(async () => null),
  addIdentityReferenceFile: vi.fn(async (): Promise<AddReferenceOutcome> => ({
    outcome: "added",
    id: 1,
  })),
  addIdentityReferenceFace: vi.fn(async () => 1),
  linkPersonIdentity: vi.fn(async (person: number) => person),
  suggestIdentities: vi.fn<(person: number, q: string) => Promise<IdentitySuggestion[]>>(
    async () => [],
  ),
  exportIdentities: vi.fn(async () => null),
  previewIdentityImport: vi.fn(),
  sequenceOverview: vi.fn<(scope: SequenceScope) => Promise<SequenceOverview>>(async () => ({
    sessions: [],
    groups: [],
    undated: [],
    suggestions: [],
    total: 0,
    gapMin: 120,
  })),
  sequencePage: vi.fn<
    (scope: SequenceScope, after: SequenceCursor | null, limit: number) => Promise<SequencePage>
  >(async () => ({ items: [], next: null })),
  sequenceSessionIds: vi.fn<(scope: SequenceScope, session: number) => Promise<number[]>>(
    async () => [],
  ),
  sequenceIds: vi.fn<(scope: SequenceScope) => Promise<number[]>>(async () => []),
  setSessionGap: vi.fn(async () => null),
  setGroupDisplay: vi.fn(async () => 1),
  setGroupShift: vi.fn(async () => 1),
  previewGroupClock: vi.fn<
    (
      group: number,
      tz: string | null,
      offset: number | null,
      shift: number,
    ) => Promise<ClockPreview[]>
  >(async () => []),
  proposeAlignment: vi.fn<(reference: number, other: number) => Promise<Alignment>>(),
  placeUndated: vi.fn(async () => 0),
  ignoreTzSuggestion: vi.fn(async () => null),
  listTimezones: vi.fn(async () => ["Europe/Rome"]),
  sequenceExportDefaults: vi.fn(async (): Promise<ExportDefaults> => ({
    template: "{seq:04d}_{date}_{time}_{tag}",
    tagSet: 2,
  })),
  sequenceExportPreview: vi.fn<
    (options: SequenceExportOptions, limit: number) => Promise<NamePreviewResult>
  >(async () => ({ names: [], errors: [] })),
  sequenceExportPreflight: vi.fn<(options: SequenceExportOptions) => Promise<ExportPreflight>>(),
  startSequenceExport: vi.fn(async () => 77),
  importIdentities: vi.fn(),
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
  addCategory: vi.fn(),
  categoryUsage: vi.fn(),
  deleteCategory: vi.fn(),
  saveTaxonomy: vi.fn(async () => null),
  onboardingDone: vi.fn(async () => true),
  completeOnboarding: vi.fn(async () => null),
  exportDiagnostics: vi.fn(async () => null),
  openLogs: vi.fn(async () => null),
  retryFailed: vi.fn(async () => []),
  listFolders: vi.fn<(sourceId: number, parent: string | null) => Promise<FolderNode[]>>(
    async () => [],
  ),
  suggestPlaces: vi.fn<(prefix: string) => Promise<PlaceSuggestion[]>>(async () => []),
  renamePlace: vi.fn(async () => ({ changed: 0, requested: 0 })),
  recomputePlaces: vi.fn(async () => 0),
  reanalyzeBelowFloor: vi.fn(async (): Promise<number | null> => 9),
  openWith: vi.fn<(ids: number[], target: OpenTarget, confirmed: boolean) => Promise<OpenOutcome>>(
    async (ids) => ({ opened: ids.length, needsConfirmation: false }),
  ),
  previewThresholds:
    vi.fn<(thresholds: AiThresholds, taxonomy: Taxonomy | null) => Promise<ThresholdImpact>>(),
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
