// Mock del modulo lib/ipc: eventi pilotabili dai test e comandi spia.
// Uso: vi.mock("../lib/ipc", () => ipcMockModule()) con percorso relativo al test.

import { vi } from "vitest";
import type {
  SourceSummary,
  ErrorBatch,
  JobSnapshot,
  LogLine,
  ModelStatus,
  RenameOutcome,
  IdentitiesView,
  AssignOutcome,
  GroupTarget,
  MergePreview,
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
  FacetValue,
  UpdateSummary,
  RelinkSummary,
  PeopleSections,
  GroupPage,
  PeopleSection,
  GroupSort,
  FacePage,
  FaceScope,
  SelectionBase,
  FaceSelectionMedia,
  FaceSort,
  FaceYear,
  FaceCursor,
  LocateFaceRequest,
  LocatedFace,
  OpenedReview,
  RecentSession,
  ShotDecision,
  PanelState,
  ReviewImageKey,
  ThumbnailKey,
  PrefetchReport,
  ImageMetadata,
  RawReviewExportRequest,
  RawReviewExportPreflight,
  RawExportReport,
  CameraStatus,
  CameraPresets,
  CameraDiagnostics,
  CameraLogLine,
  RawReviewDevelopRequest,
  RawReviewDeveloped,
  VariantPlan,
  DevelopReport,
  MergeEntry,
  ReviewQueue,
  Answer,
  AnswerOutcome,
  AggregationSummary,
  ClusterMetrics,
} from "../bindings";

type Cb<T> = (p: T) => void;

export const emitters = {
  boot: [] as Cb<import("../bindings").BootStatus>[],
  job: [] as Cb<JobSnapshot>[],
  resources: [] as Cb<ResourceSnapshot>[],
  log: [] as Cb<LogLine[]>[],
  errorBatch: [] as Cb<ErrorBatch>[],
  models: [] as Cb<ModelStatus[]>[],
  uiRecovered: [] as Cb<void>[],
  trimCaches: [] as Cb<void>[],
  facesChanged: [] as Cb<void>[],
  menuCommand: [] as Cb<string>[],
  sourceWatchChanged: [] as Cb<void>[],
  rawReviewCameraStatus: [] as Cb<CameraStatus>[],
  rawReviewCameraLog: [] as Cb<CameraLogLine>[],
  rawReviewCameraDiagnostics: [] as Cb<CameraDiagnostics>[],
  rawReviewDeveloped: [] as Cb<RawReviewDeveloped>[],
  updateProgress: [] as Cb<import("../bindings").UpdateProgress>[],
  updateReady: [] as Cb<import("../bindings").UpdateReady>[],
  updateFailed: [] as Cb<import("../bindings").UpdateFailed>[],
};

export function emit<K extends keyof typeof emitters>(
  kind: K,
  payload: Parameters<(typeof emitters)[K][number]>[0],
) {
  for (const cb of emitters[kind] as Cb<typeof payload>[]) cb(payload);
}

/** Informazioni sull'app di prova (app-about); i test cambiano solo i campi che servono. */
export function appInfoFixture(overrides: Record<string, unknown> = {}) {
  return {
    version: "0.5.1",
    commit: "a1b2c3d" as string | null,
    buildDate: "2026-11-23",
    profile: "release",
    os: "Windows 11 Pro",
    arch: "x86_64",
    webview: "131.0.2903.70" as string | null,
    accelerator: "DirectML",
    authors: "photo-cat contributors",
    repository: "https://github.com/federicocunico/photo-cat" as string | null,
    license: "MIT OR Apache-2.0",
    dataDir: "D:/photo-cat",
    logsDir: "D:/photo-cat/logs",
    previousCrash: null as unknown,
    ...overrides,
  };
}

export const commandsMock = {
  bootStatus: vi.fn(async () => ({
    phase: "ready",
    ready: true,
    error: null as string | null,
    logsPath: "D:/logs",
  })),
  bootRetry: vi.fn(async () => {}),
  bootChooseFolder: vi.fn(async () => null),
  appInfo: vi.fn(async () => appInfoFixture()),
  // Aggiornamenti dell'app (app-updates).
  checkUpdate: vi.fn<() => Promise<import("../bindings").UpdateInfo | null>>(async () => null),
  downloadUpdate: vi.fn<(version: string) => Promise<null>>(async () => null),
  cancelUpdateDownload: vi.fn(async () => undefined),
  installUpdate: vi.fn<(version: string) => Promise<null>>(async () => null),
  openReleasePage: vi.fn<(url: string) => Promise<null>>(async () => null),
  dismissCrashNotice: vi.fn(),
  takeUiRecovery: vi.fn(async () => null),
  reportLongTasks: vi.fn(async () => null),
  devNativeCrash: vi.fn(async () => null),
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
  errorGroups: vi.fn(async () => []),
  errorCount: vi.fn(async () => 0),
  archiveErrors: vi.fn(async () => ({ affected: 0, activeCount: 0 })),
  unarchiveErrors: vi.fn(async () => ({ affected: 0, activeCount: 0 })),
  logTail: vi.fn(async () => []),
  setLogLevel: vi.fn(async () => null),
  listSources: vi.fn(async () => [] as SourceSummary[]),
  acknowledgeSourceWarnings: vi.fn(async () => null),
  sourceWatchStatus: vi.fn(async () => [] as import("../bindings").SourceWatchStatus[]),
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
  libraryPosition: vi.fn(async () => null as number | null),
  libraryCount: vi.fn(),
  libraryFacets: vi.fn(),
  libraryFacetValues: vi.fn(async () => ({ values: [] as FacetValue[], total: 0 })),
  librarySelection: vi.fn(),
  libraryGridCount: vi.fn(async () => 0),
  similarGroup: vi.fn(async () => null),
  similarRemove: vi.fn(async () => null),
  similarGroupTogether: vi.fn(async () => null),
  similarDissolve: vi.fn(async () => null),
  similarSetCover: vi.fn(async () => null),
  similarGroupMembers: vi.fn(async () => ({ items: [], next: null, prev: null })),
  similarNonCovers: vi.fn(async () => [] as number[]),
  similarPreview: vi.fn(async () => 0),
  similarPending: vi.fn(async () => 0),
  startSimilarAnalysis: vi.fn(async () => 1),
  mediaDetail: vi.fn(),
  listCatalogs: vi.fn(async () => []),
  trashImpact: vi.fn(async () => [] as { name: string; items: number }[]),
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
  peopleSections: vi.fn(async (): Promise<PeopleSections> => ({
    review: 0,
    people: 0,
    ignored: 0,
    singles: 0,
    discarded: 0,
  })),
  listGroups: vi.fn<
    (
      section: PeopleSection,
      query: string | null,
      sort: GroupSort,
      after: number | null,
      limit: number | null,
      maxPhotos: number | null,
    ) => Promise<GroupPage>
  >(async () => ({ items: [], next: null, total: 0 })),
  groupSelectionIds: vi.fn<
    (section: PeopleSection, query: string | null, maxPhotos: number | null) => Promise<number[]>
  >(async () => []),
  assignGroups: vi.fn<(groups: number[], target: GroupTarget) => Promise<AssignOutcome>>(
    async (groups) => ({
      person: 1,
      created: false,
      groups: groups.length,
      faces: groups.length,
      skippedFaces: 0,
    }),
  ),
  // Di default cerca il gruppo nelle pagine simulate da `listGroups`.
  getGroup: vi.fn(async (id: number) => {
    for (const section of ["review", "people", "ignored"] as const) {
      const page = await commandsMock.listGroups(section, null, "photos_desc", null, 200, null);
      const found = page?.items?.find((g) => g.id === id);
      if (found) return found;
    }
    return null;
  }),
  groupFaceYears: vi.fn<(scope: FaceScope) => Promise<FaceYear[]>>(async () => []),
  groupFaces: vi.fn<
    (
      scope: FaceScope,
      sort: FaceSort,
      after: FaceCursor | null,
      limit: number | null,
    ) => Promise<FacePage>
  >(async () => ({
    items: [],
    next: null,
    total: 0,
    goodCount: 0,
    discardedCount: 0,
  })),
  faceSelection: vi.fn<(scope: FaceScope) => Promise<SelectionBase>>(async () => ({
    count: 0,
    maxId: 0,
  })),
  faceSelectionIds: vi.fn<(scope: FaceScope, maxId: number, except: number[]) => Promise<number[]>>(
    async () => [],
  ),
  faceSelectionMedia: vi.fn<
    (scope: FaceScope, maxId: number, except: number[]) => Promise<FaceSelectionMedia>
  >(async () => ({ present: [], missing: [] })),
  locateFace: vi.fn<(request: LocateFaceRequest) => Promise<LocatedFace>>(),
  personFaces: vi.fn(async () => []),
  correctPerson: vi.fn(async () => null),
  purgeFaces: vi.fn(async () => null),
  saveCatalog: vi.fn(async () => 1),
  deleteCatalog: vi.fn(async () => null),
  deleteCatalogs: vi.fn(async () => ({ deleted: 0, skipped: [] as number[] })),
  startBulk: vi.fn(async () => 1),
  undoBulk: vi.fn(async () => null),
  undoPeople: vi.fn(async () => null),
  listMerges: vi.fn<(person: number | null, limit: number | null) => Promise<MergeEntry[]>>(
    async () => [],
  ),
  undoMerge: vi.fn<(id: number) => Promise<number>>(async () => 0),
  reviewQueue: vi.fn<(limit: number | null) => Promise<ReviewQueue>>(async () => ({
    items: [],
    open: 0,
    unnamedGroups: 0,
  })),
  answerQuestion: vi.fn<
    (question: number, answer: Answer, overrideConflict: boolean) => Promise<AnswerOutcome>
  >(async () => ({ mergedGroups: 0, faces: 0, constrainedGroups: 0, conflict: null })),
  answerQuestions: vi.fn<
    (
      person: number,
      confirm: number[],
      reject: number[],
      overrideConflict: boolean,
    ) => Promise<AnswerOutcome>
  >(async () => ({ mergedGroups: 0, faces: 0, constrainedGroups: 0, conflict: null })),
  aggregationSummary: vi.fn<() => Promise<AggregationSummary | null>>(async () => null),
  dismissAggregationSummary: vi.fn(async () => null),
  faceClusterMetrics: vi.fn<() => Promise<ClusterMetrics | null>>(async () => null),
  lastBulkUndo: vi.fn(async () => null),
  collectDuplicates: vi.fn(async () => 0),
  startPhotoQuality: vi.fn(async () => 1),
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
  listIdentities: vi.fn<(q: string | null, maxPhotos: number | null) => Promise<IdentitiesView>>(
    async () => ({
      identities: [],
      groups: [],
      storePath: "",
      defaultThreshold: 0.363,
    }),
  ),
  identitySelectionUids: vi.fn<(q: string | null, maxPhotos: number | null) => Promise<string[]>>(
    async () => [],
  ),
  saveIdentity: vi.fn(async () => "uid-1"),
  deleteIdentity: vi.fn(async () => null),
  mergeIdentities: vi.fn(async () => null),
  mergeIdentitiesMany: vi.fn<(from: string[], into: string) => Promise<number>>(
    async (from) => from.length,
  ),
  mergeIdentitiesPreview: vi.fn<(from: string[], into: string) => Promise<MergePreview>>(
    async (from) => ({
      identities: from.length,
      photos: 0,
      referenceFaces: 0,
      withoutPhotos: 0,
      names: [],
    }),
  ),
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
  sequenceSelection: vi.fn<(scope: SequenceScope) => Promise<SelectionBase>>(async () => ({
    count: 0,
    maxId: 0,
  })),
  sequenceSelectionIds: vi.fn<
    (scope: SequenceScope, maxId: number, except: number[]) => Promise<number[]>
  >(async () => []),
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
  rawReviewCacheInfo: vi.fn(async () => ({
    available: true,
    usedBytes: 0,
    limitBytes: 20 * 1024 ** 3,
  })),
  rawReviewClearCache: vi.fn(async () => 0),
  rawReviewExportPreflight:
    vi.fn<(request: RawReviewExportRequest) => Promise<RawReviewExportPreflight>>(),
  rawReviewPreviewNames: vi.fn<
    (request: RawReviewExportRequest, limit: number) => Promise<NamePreviewResult>
  >(async () => ({ names: [], errors: [] })),
  startRawReviewExport: vi.fn<(request: RawReviewExportRequest) => Promise<number>>(async () => 88),
  rawReviewExportReport: vi.fn<(jobId: number) => Promise<RawExportReport | null>>(
    async () => null,
  ),
  // Sviluppo dalla fotocamera: di norma "non disponibile" (Linux/macOS); i test lo cambiano.
  rawReviewCameraStatus: vi.fn<() => Promise<CameraStatus>>(async () => ({
    available: false,
    state: "disconnected",
    model: null,
    firmware: null,
    message: "Disponibile solo su Windows",
    simulated: false,
  })),
  rawReviewCameraPresets: vi.fn<() => Promise<CameraPresets>>(async () => ({
    presets: [],
    selectedIds: [],
  })),
  rawReviewCameraDiagnostics: vi.fn<() => Promise<CameraDiagnostics>>(),
  rawReviewCameraLog: vi.fn(async () => [] as CameraLogLine[]),
  rawReviewDevelopPlan: vi.fn<(request: RawReviewDevelopRequest) => Promise<VariantPlan>>(),
  startRawReviewDevelop: vi.fn<(request: RawReviewDevelopRequest) => Promise<number>>(
    async () => 90,
  ),
  rawReviewDevelopReport: vi.fn<(jobId: number) => Promise<DevelopReport | null>>(async () => null),
  rawReviewOpen: vi.fn<(root: string) => Promise<OpenedReview>>(),
  rawReviewOpenFujiview: vi.fn<(path: string) => Promise<OpenedReview>>(),
  rawReviewRecent: vi.fn(async () => [] as RecentSession[]),
  rawReviewForget: vi.fn(async () => null),
  rawReviewSave: vi.fn(async () => null),
  rawReviewClose: vi.fn(async () => true),
  rawReviewSetWinner:
    vi.fn<(id: string, shot: string, preset: string | null) => Promise<ShotDecision>>(),
  rawReviewSetReviewed:
    vi.fn<(id: string, shot: string, reviewed: boolean) => Promise<ShotDecision>>(),
  rawReviewSetLastShot: vi.fn(async () => null),
  rawReviewSetRating: vi.fn<(id: string, shot: string, rating: number) => Promise<ShotDecision>>(),
  rawReviewThumbnails: vi.fn<(id: string, shots: string[]) => Promise<ThumbnailKey[]>>(
    async () => [],
  ),
  rawReviewTogglePreset: vi.fn<(id: string, preset: string) => Promise<PanelState>>(),
  rawReviewMovePreset: vi.fn<(id: string, preset: string, to: number) => Promise<PanelState>>(),
  rawReviewShowAllPresets: vi.fn<(id: string) => Promise<PanelState>>(),
  rawReviewImageKeys: vi.fn<(id: string, shot: string, edge: number) => Promise<ReviewImageKey[]>>(
    async () => [],
  ),
  rawReviewPrefetch: vi.fn<
    (id: string, center: string, presets: string[]) => Promise<PrefetchReport>
  >(async () => ({ queued: 0, cancelled: 0 })),
  rawReviewMetadata: vi.fn(async (): Promise<ImageMetadata> => ({
    sections: [],
    gps: null,
    recipe: null,
  })),
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
  openDataDir: vi.fn(async () => null),
  setAppMenu: vi.fn(async () => null),
  thirdPartyLicenses: vi.fn(async () => ({
    components: [
      {
        name: "libheif",
        version: "1.23.7",
        license: "LGPL-3.0-or-later",
        kind: "native",
        text: "lgpl",
        sources: "native-sources/libheif-1.23.7.tar.gz" as string | null,
      },
      {
        name: "libde265",
        version: "1.1.3",
        license: "LGPL-3.0-or-later",
        kind: "native",
        text: "lgpl",
        sources: "native-sources/libde265-1.1.3.tar.gz" as string | null,
      },
      {
        name: "serde",
        version: "1.0.229",
        license: "MIT OR Apache-2.0",
        kind: "rust",
        text: "mit",
        sources: null as string | null,
      },
      {
        name: "react",
        version: "18.3.1",
        license: "MIT",
        kind: "ui",
        text: "mit",
        sources: null as string | null,
      },
    ],
    texts: { lgpl: "GNU LESSER GENERAL PUBLIC LICENSE", mit: "MIT License" } as Record<
      string,
      string
    >,
    root: "C:\\Program Files\\photo-cat",
  })),
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
    EVENTS: {
      facesChanged: "faces://changed",
      sourceWatchChanged: "sources://watch-changed",
      rawReviewCameraStatus: "raw-review://camera-status",
      rawReviewCameraLog: "raw-review://camera-log",
      rawReviewCameraDiagnostics: "raw-review://camera-diagnostics",
      rawReviewDeveloped: "raw-review://developed",
    },
    events: {
      onBootPhase: reg(emitters.boot),
      onBootReady: reg(emitters.boot),
      onJobProgress: reg(emitters.job),
      onResources: reg(emitters.resources),
      onLogLines: reg(emitters.log),
      onErrorBatch: reg(emitters.errorBatch),
      onModels: reg(emitters.models),
      onUiRecovered: reg(emitters.uiRecovered),
      onTrimCaches: reg(emitters.trimCaches),
      onFacesChanged: reg(emitters.facesChanged),
      onMenuCommand: reg(emitters.menuCommand),
      onSourceWatchChanged: reg(emitters.sourceWatchChanged),
      onRawReviewCameraStatus: reg(emitters.rawReviewCameraStatus),
      onRawReviewCameraLog: reg(emitters.rawReviewCameraLog),
      onRawReviewCameraDiagnostics: reg(emitters.rawReviewCameraDiagnostics),
      onRawReviewDeveloped: reg(emitters.rawReviewDeveloped),
      onUpdateProgress: reg(emitters.updateProgress),
      onUpdateReady: reg(emitters.updateReady),
      onUpdateFailed: reg(emitters.updateFailed),
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
    memoryPaused: false,
    pauseReason: null,
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
    uiTrim: null,
  };
}
