// Dialogo "Esporta in un nuovo catalogo" delle Varianti RAW: copia i JPEG scelti in una cartella
// e li mette in un catalogo della libreria. Il modulo non conta nulla da solo: conteggi, esclusi,
// nomi, conflitti e cosa farà l'import arrivano dalla verifica preliminare del backend
// (`rawReviewExportPreflight`), richiesta con un breve ritardo mentre l'utente cambia le opzioni.
// L'esportazione è un'attività: l'avanzamento sta in Attività, qui il riepilogo finale.
import { keepPreviousData, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, FolderPicker, FormField, InlineAlert } from "../../components/ui";
import {
  commands,
  isTauri,
  type ExportBlock,
  type JobSnapshot,
  type RawExportReport,
  type RawReviewExportPreflight,
  type RawReviewExportRequest,
  type TemplateError,
} from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { currentPhase } from "../../lib/jobs";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";

const DEST_KEY = "photo-cat.rawReviewExportDestination";
const DEBOUNCE_MS = 250;

/** Segnaposto del modello dei nomi: (testo da inserire, chiave della spiegazione). */
const TOKENS: Array<[string, string]> = [
  ["{seq}", "rawExport.tokens.seq"],
  ["{seq:04d}", "rawExport.tokens.seq4"],
  ["{date}", "rawExport.tokens.date"],
  ["{time}", "rawExport.tokens.time"],
  ["{time_full}", "rawExport.tokens.timeFull"],
  ["{datetime}", "rawExport.tokens.datetime"],
  ["{device}", "rawExport.tokens.device"],
  ["{orig}", "rawExport.tokens.orig"],
  ["{preset}", "rawExport.tokens.preset"],
];

const BLOCK_KEYS: Record<ExportBlock, string> = {
  no_photos: "rawExport.block.noPhotos",
  template_invalid: "rawExport.block.templateInvalid",
  catalog_name_empty: "rawExport.block.catalogNameEmpty",
  catalog_name_taken: "rawExport.block.catalogNameTaken",
  catalog_not_addable: "rawExport.block.catalogNotAddable",
  destination_invalid: "rawExport.block.destinationInvalid",
  destination_in_review: "rawExport.block.destinationInReview",
};

const TERMINAL = ["completed", "completed_with_errors", "cancelled", "interrupted", "failed"];

function readDestination(): string {
  try {
    return localStorage.getItem(DEST_KEY) ?? "";
  } catch {
    return "";
  }
}

function saveDestination(path: string) {
  try {
    localStorage.setItem(DEST_KEY, path);
  } catch {
    // Solo una comodità: senza archivio il percorso non viene ricordato.
  }
}

/** Valore che segue `value` con un piccolo ritardo (richieste al backend mentre si scrive). */
function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

/** Apre il catalogo in Libreria (come "Apri" in Cataloghi). */
function openCatalog(id: number) {
  const lib = useLibrary.getState();
  lib.resetFilter();
  lib.setSelection([], null);
  lib.setFilter({ catalogId: id });
  useLayout.getState().openModule({ module: "library" });
}

type Conflict = "add" | "rename" | null;

export function ExportToCatalogDialog({
  open,
  onOpenChange,
  sessionId,
  sessionName,
  presets,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string;
  /** Nome della sessione: nome predefinito del catalogo. */
  sessionName: string;
  /** Preset della sessione, per il filtro. */
  presets: string[];
}) {
  const { t } = useTranslation();
  const [destination, setDestination] = useState(readDestination);
  const [catalogName, setCatalogName] = useState(sessionName);
  const [minRating, setMinRating] = useState(0);
  const [onlyPresets, setOnlyPresets] = useState<string[] | null>(null);
  const [rename, setRename] = useState(false);
  const [overwrite, setOverwrite] = useState(false);
  const [presetAsTag, setPresetAsTag] = useState(true);
  const [conflict, setConflict] = useState<Conflict>(null);
  const [jobId, setJobId] = useState<number | null>(null);
  const [startError, setStartError] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const templateRef = useRef<HTMLInputElement>(null);

  // Il modello predefinito segue il nome del catalogo finché l'utente non lo scrive a mano.
  const [customTemplate, setCustomTemplate] = useState<string | null>(null);
  const effectiveTemplate = rename ? (customTemplate ?? `${catalogName.trim()}_{seq:04d}`) : null;

  const request: RawReviewExportRequest = useMemo(
    () => ({
      sessionId,
      destination: destination.trim(),
      minRating,
      presets: onlyPresets,
      template: effectiveTemplate,
      catalogName: catalogName.trim(),
      existingCatalog: conflict === "add" ? "add" : null,
      overwrite,
      presetAsTag,
    }),
    [
      sessionId,
      destination,
      minRating,
      onlyPresets,
      effectiveTemplate,
      catalogName,
      conflict,
      overwrite,
      presetAsTag,
    ],
  );
  const debounced = useDebounced(request, DEBOUNCE_MS);
  const settled = debounced === request;
  const preflight = useQuery({
    queryKey: ["raw-review-export-preflight", debounced],
    queryFn: () => commands.rawReviewExportPreflight(debounced),
    enabled: isTauri() && open && jobId === null,
    placeholderData: keepPreviousData,
    retry: false,
    // I file della destinazione e le sorgenti possono cambiare fuori dall'app.
    staleTime: 0,
  });
  const pre = preflight.data;
  const checking = !settled || preflight.isFetching;

  const changeName = (value: string) => {
    setCatalogName(value);
    // Un altro nome è un'altra domanda: la scelta sul catalogo esistente non vale più.
    setConflict(null);
  };

  const insertToken = (token: string) => {
    const el = templateRef.current;
    const base = effectiveTemplate ?? "";
    const a = el?.selectionStart ?? base.length;
    const b = el?.selectionEnd ?? base.length;
    setCustomTemplate(base.slice(0, a) + token + base.slice(b));
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(a + token.length, a + token.length);
    });
  };

  const choosePreset = (preset: string, on: boolean) => {
    const current = onlyPresets ?? presets;
    const next = on ? [...current, preset] : current.filter((p) => p !== preset);
    // Tutti i preset scelti equivalgono a nessun filtro.
    setOnlyPresets(next.length === presets.length ? null : next);
  };

  const blockReason = (): string | undefined => {
    if (!pre) return t("rawExport.checking");
    if (checking) return t("rawExport.checking");
    if (pre.canStart) return undefined;
    if (pre.block === "catalog_not_addable" && pre.catalogConflict)
      return t(BLOCK_KEYS.catalog_not_addable, { name: pre.catalogConflict.name });
    return pre.block ? t(BLOCK_KEYS[pre.block]) : t("rawExport.block.generic");
  };
  const reason = blockReason();

  const start = async () => {
    setStartError("");
    try {
      const id = await commands.startRawReviewExport(request);
      saveDestination(request.destination);
      setJobId(id);
    } catch (e) {
      setStartError((e as { detail?: string }).detail ?? String(e));
    }
  };

  const reset = () => {
    setJobId(null);
    setStartError("");
    setConflict(null);
  };

  const running = jobId !== null;
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("rawExport.title")}
      description={running ? undefined : t("rawExport.intro")}
      width={680}
      footer={
        running ? undefined : (
          <>
            <Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>
            <Button
              variant="primary"
              disabledReason={reason}
              onClick={start}
              data-testid="raw-export-start"
            >
              {t("rawExport.start", { count: pre?.total ?? 0 })}
            </Button>
          </>
        )
      }
    >
      {running ? (
        <Progress
          jobId={jobId}
          onClose={() => onOpenChange(false)}
          onAgain={reset}
          onOpened={() => onOpenChange(false)}
        />
      ) : (
        <div className="rx-form">
          <FormField label={t("rawExport.catalogName")} required>
            <input
              ref={nameRef}
              className="field"
              value={catalogName}
              spellCheck={false}
              onChange={(e) => changeName(e.target.value)}
            />
          </FormField>
          {pre?.catalogConflict && (
            <ConflictChoice
              name={pre.catalogConflict.name}
              canAdd={pre.catalogConflict.canAdd}
              value={conflict}
              onAdd={() => setConflict("add")}
              onRename={() => {
                setConflict("rename");
                nameRef.current?.focus();
                nameRef.current?.select();
              }}
            />
          )}

          <FolderPicker
            label={t("rawExport.destination")}
            value={destination}
            onChange={setDestination}
            hint={t("rawExport.destinationHint")}
          />
          {pre?.import && (
            <p className="rx-note" data-testid="raw-export-import">
              {pre.import.kind === "newSource"
                ? t("rawExport.importNew", { path: pre.import.path })
                : t("rawExport.importUpdate", { folder: pre.import.folder })}
            </p>
          )}

          <fieldset className="rx-group">
            <legend>{t("rawExport.which")}</legend>
            <FormField label={t("rawExport.minRating")}>
              <select
                className="field rx-select"
                value={minRating}
                onChange={(e) => setMinRating(Number(e.target.value))}
              >
                {[0, 1, 2, 3, 4, 5].map((n) => (
                  <option key={n} value={n}>
                    {n === 0 ? t("rawExport.allChoices") : t("rawExport.atLeast", { count: n })}
                  </option>
                ))}
              </select>
            </FormField>
            {pre && (
              <ul className="rx-counts" aria-label={t("rawExport.byRating")}>
                {pre.byRating.map((count, n) => (
                  <li key={n} className={n < minRating ? "rx-count-off" : undefined}>
                    <span>
                      {n === 0 ? t("rawExport.noStars") : t("rawExport.stars", { count: n })}
                    </span>
                    <strong>{fmtInt(count)}</strong>
                  </li>
                ))}
              </ul>
            )}
            {presets.length > 1 && (
              <fieldset className="rx-presets">
                <legend>{t("rawExport.presets")}</legend>
                <div className="rx-preset-list">
                  {presets.map((p) => (
                    <label key={p} className="rx-check">
                      <input
                        type="checkbox"
                        checked={onlyPresets === null || onlyPresets.includes(p)}
                        onChange={(e) => choosePreset(p, e.target.checked)}
                      />
                      <span>{p}</span>
                    </label>
                  ))}
                </div>
              </fieldset>
            )}
            {pre && (
              <>
                <p className="rx-total" role="status" data-testid="raw-export-total">
                  {t("rawExport.total", { count: pre.total })}
                </p>
                <Excluded pre={pre} />
              </>
            )}
          </fieldset>

          <fieldset className="rx-group">
            <legend>{t("rawExport.names")}</legend>
            <label className="rx-check">
              <input
                type="radio"
                name="rx-names"
                checked={!rename}
                onChange={() => setRename(false)}
              />
              <span>{t("rawExport.keepNames")}</span>
            </label>
            <label className="rx-check">
              <input
                type="radio"
                name="rx-names"
                checked={rename}
                onChange={() => setRename(true)}
              />
              <span>{t("rawExport.renameNames")}</span>
            </label>
            {rename && (
              <div className="rx-template">
                <FormField label={t("rawExport.template")}>
                  <input
                    ref={templateRef}
                    className="field mono"
                    value={effectiveTemplate ?? ""}
                    spellCheck={false}
                    onChange={(e) => setCustomTemplate(e.target.value)}
                  />
                </FormField>
                <div className="rx-tokens" role="group" aria-label={t("rawExport.tokensLabel")}>
                  {TOKENS.map(([token, help]) => (
                    <button
                      key={token}
                      type="button"
                      className="chip mono"
                      title={t(help)}
                      aria-label={t("rawExport.insertToken", { token, help: t(help) })}
                      onClick={() => insertToken(token)}
                    >
                      {token}
                    </button>
                  ))}
                </div>
                {pre && pre.templateErrors.length > 0 && (
                  <p role="alert" className="rx-error">
                    {pre.templateErrors.map((e) => templateError(t, e)).join(" · ")}
                  </p>
                )}
                {pre && pre.templateErrors.length === 0 && pre.preview.length > 0 && (
                  <table className="table rx-preview" aria-label={t("rawExport.preview")}>
                    <thead>
                      <tr>
                        <th>{t("rawExport.original")}</th>
                        <th aria-hidden="true" />
                        <th>{t("rawExport.newName")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pre.preview.map((p) => (
                        <tr key={p.original + p.name}>
                          <td className="mono muted">{p.original}</td>
                          <td aria-hidden="true">→</td>
                          <td className="mono">{p.name}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
                <p className="muted rx-small">{t("rawExport.templateHelp")}</p>
              </div>
            )}
          </fieldset>

          <fieldset className="rx-group">
            <legend>{t("rawExport.options")}</legend>
            <label className="rx-check">
              <input
                type="checkbox"
                checked={overwrite}
                onChange={(e) => setOverwrite(e.target.checked)}
              />
              <span>
                {t("rawExport.overwrite")}
                <br />
                <span className="muted rx-small">
                  {overwrite ? t("rawExport.overwriteOn") : t("rawExport.overwriteOff")}
                </span>
              </span>
            </label>
            {pre && pre.existingFiles > 0 && (
              <p className="rx-note" data-testid="raw-export-existing">
                {t(overwrite ? "rawExport.existingOverwrite" : "rawExport.existingSkip", {
                  count: pre.existingFiles,
                })}
              </p>
            )}
            <label className="rx-check">
              <input
                type="checkbox"
                checked={presetAsTag}
                onChange={(e) => setPresetAsTag(e.target.checked)}
              />
              <span>
                {t("rawExport.presetAsTag")}
                <br />
                <span className="muted rx-small">{t("rawExport.presetAsTagHelp")}</span>
              </span>
            </label>
          </fieldset>

          {reason && !checking && (
            <p className="rx-block" role="status" data-testid="raw-export-block">
              {reason}
            </p>
          )}
          <p className="muted rx-small">{t("rawExport.safety")}</p>
          {startError && (
            <InlineAlert tone="danger" title={t("rawExport.startFailed")}>
              {startError}
            </InlineAlert>
          )}
        </div>
      )}
    </Dialog>
  );
}

function templateError(t: (k: string, o?: Record<string, unknown>) => string, e: TemplateError) {
  switch (e.kind) {
    case "unknown_token":
      return t("rawExport.errors.unknownToken", { token: e.text });
    case "unavailable_token":
      return t("rawExport.errors.unavailableToken", { token: e.text });
    case "invalid_char":
      return t("rawExport.errors.invalidChar", { char: e.text });
    case "unclosed":
      return t("rawExport.errors.unclosed");
    default:
      return t("rawExport.errors.empty");
  }
}

/** Scatti che non verranno esportati, con il motivo. */
function Excluded({ pre }: { pre: RawReviewExportPreflight }) {
  const { t } = useTranslation();
  const rows = [
    pre.noChoice > 0 && t("rawExport.noChoice", { count: pre.noChoice }),
    pre.notJpeg > 0 && t("rawExport.notJpeg", { count: pre.notJpeg }),
    pre.missing > 0 && t("rawExport.missing", { count: pre.missing }),
  ].filter(Boolean) as string[];
  if (rows.length === 0) return null;
  return (
    <div className="rx-excluded" data-testid="raw-export-excluded">
      <strong>{t("rawExport.excluded")}</strong>
      <ul>
        {rows.map((r) => (
          <li key={r}>{r}</li>
        ))}
      </ul>
      {pre.notJpeg > 0 && <p className="muted rx-small">{t("rawExport.notJpegHelp")}</p>}
    </div>
  );
}

/** Il nome del catalogo esiste già: aggiungere le foto a quello o cambiare nome, prima di partire. */
function ConflictChoice({
  name,
  canAdd,
  value,
  onAdd,
  onRename,
}: {
  name: string;
  canAdd: boolean;
  value: Conflict;
  onAdd: () => void;
  onRename: () => void;
}) {
  const { t } = useTranslation();
  return (
    <InlineAlert tone="warn" title={t("rawExport.conflictTitle", { name })}>
      <p>{canAdd ? t("rawExport.conflictBody") : t("rawExport.conflictNotAddable")}</p>
      <div className="rx-conflict-actions" role="group" aria-label={t("rawExport.conflictChoice")}>
        {canAdd && (
          <Button aria-pressed={value === "add"} onClick={onAdd}>
            {t("rawExport.conflictAdd")}
          </Button>
        )}
        <Button aria-pressed={value === "rename"} onClick={onRename}>
          {t("rawExport.conflictRename")}
        </Button>
      </div>
      {value === "add" && <p role="status">{t("rawExport.conflictAdding", { name })}</p>}
      {value === "rename" && <p role="status">{t("rawExport.conflictRenaming")}</p>}
    </InlineAlert>
  );
}

/** Avanzamento (nelle Attività) e riepilogo finale dell'esportazione avviata. */
function Progress({
  jobId,
  onClose,
  onAgain,
  onOpened,
}: {
  jobId: number;
  onClose: () => void;
  onAgain: () => void;
  onOpened: () => void;
}) {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const job: JobSnapshot | undefined = useLive((s) => s.jobs[jobId]);
  const finished = job !== undefined && TERMINAL.includes(job.state);
  const [report, setReport] = useState<RawExportReport | null>(null);
  const [gaveUp, setGaveUp] = useState(false);

  // Il riepilogo esiste solo a lavoro finito: si chiede appena lo stato lo dice, e con un po' di
  // pazienza se il backend lo scrive un istante dopo.
  useEffect(() => {
    if (!finished || report) return;
    let alive = true;
    let tries = 0;
    const ask = async () => {
      try {
        const r = await commands.rawReviewExportReport(jobId);
        if (!alive) return;
        if (r) {
          setReport(r);
          void qc.invalidateQueries({ queryKey: ["catalogs"] });
          void qc.invalidateQueries({ queryKey: ["sources"] });
          return;
        }
      } catch {
        // Si riprova sotto.
      }
      tries += 1;
      if (!alive) return;
      if (tries >= 8) setGaveUp(true);
      else timer = setTimeout(ask, 400);
    };
    let timer: ReturnType<typeof setTimeout> | undefined;
    void ask();
    return () => {
      alive = false;
      if (timer) clearTimeout(timer);
    };
  }, [finished, report, jobId, qc]);

  const phase = job ? currentPhase(job) : undefined;
  const openJobs = (
    <Button
      variant="ghost"
      onClick={() => {
        useLayout.getState().openModule({ module: "jobs", target: jobId });
        onClose();
      }}
    >
      {t("rawExport.openJobs")}
    </Button>
  );

  if (!report) {
    if (finished && gaveUp)
      return (
        <div className="rx-progress">
          <InlineAlert
            tone={job?.state === "failed" ? "danger" : "warn"}
            title={t("rawExport.noReport")}
            action={openJobs}
          >
            {job?.state === "failed" && job.error
              ? job.error
              : job?.state === "completed" || job?.state === "completed_with_errors"
                ? t("rawExport.noReportDone")
                : t("rawExport.noReportBody")}
          </InlineAlert>
          <div className="rx-final">
            <Button onClick={onClose}>{t("common.close")}</Button>
          </div>
        </div>
      );
    return (
      <div className="rx-progress" role="status">
        <p>
          <strong>{t("rawExport.started")}</strong>
        </p>
        <progress
          className="rx-bar"
          aria-label={t("rawExport.progress")}
          max={100}
          value={job?.overallPercent ?? undefined}
        />
        {phase && (
          <p className="muted">{t(`jobs.phases.${phase.name}`, { defaultValue: phase.name })}</p>
        )}
        <p className="muted rx-small">{t("rawExport.startedHelp")}</p>
        <div className="rx-final">
          {openJobs}
          <Button onClick={onClose}>{t("common.close")}</Button>
        </div>
      </div>
    );
  }

  const done = report.failed + report.skipped + report.exported;
  return (
    <div className="rx-progress" data-testid="raw-export-report">
      <h3 className="rx-report-title" role="status">
        {report.failure
          ? t("rawExport.reportStopped")
          : report.cancelled
            ? t("rawExport.reportCancelled")
            : report.failed > 0 || report.unresolved > 0
              ? t("rawExport.reportWithErrors")
              : t("rawExport.reportDone")}
      </h3>
      {report.failure && (
        <InlineAlert tone="danger" title={t("rawExport.stoppedTitle")}>
          {failureReason(report.failure)}
        </InlineAlert>
      )}
      <ul className="rx-report-list">
        <li>{t("rawExport.reportExported", { count: report.exported })}</li>
        {report.skipped > 0 && <li>{t("rawExport.reportSkipped", { count: report.skipped })}</li>}
        {report.failed > 0 && <li>{t("rawExport.reportFailed", { count: report.failed })}</li>}
        {done === 0 && !report.cancelled && !report.failure && (
          <li>{t("rawExport.reportNothing")}</li>
        )}
        {report.catalog && (
          <li>
            {t(report.catalog.created ? "rawExport.catalogCreated" : "rawExport.catalogAdded", {
              name: report.catalog.name,
              count: report.catalog.photos,
            })}
          </li>
        )}
        {report.unresolved > 0 && (
          <li>{t("rawExport.reportUnresolved", { count: report.unresolved })}</li>
        )}
      </ul>
      {(report.cancelled || report.failure) && !report.catalog && done > 0 && (
        <p className="rx-note">{t("rawExport.cancelledNote")}</p>
      )}
      {report.errors.length > 0 && (
        <div>
          <strong>{t("rawExport.errorsTitle")}</strong>
          <ul className="rx-errors" aria-label={t("rawExport.errorsTitle")}>
            {report.errors.map((e) => (
              <li key={e.sourcePath + e.name}>
                <span className="mono">{e.name}</span>
                <span className="muted"> — {errorReason(e)}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="rx-final">
        <Button variant="ghost" onClick={onAgain}>
          {t("rawExport.again")}
        </Button>
        <Button onClick={onClose}>{t("common.close")}</Button>
        {report.catalog && (
          <Button
            variant="primary"
            onClick={() => {
              openCatalog(report.catalog!.id);
              onOpened();
            }}
          >
            {t("rawExport.openCatalog")}
          </Button>
        )}
      </div>
    </div>
  );

  function errorReason(e: RawExportReport["errors"][number]): string {
    const key = `errors.${e.code.toLowerCase()}`;
    return i18n.exists(key) ? t(key) : e.reason;
  }

  function failureReason(f: NonNullable<RawExportReport["failure"]>): string {
    const key = `errors.${f.code.toLowerCase()}`;
    // Il motivo tecnico resta visibile: dice quale cartella o quale passo non è riuscito.
    return i18n.exists(key) ? `${t(key)} (${f.reason})` : f.reason;
  }
}
