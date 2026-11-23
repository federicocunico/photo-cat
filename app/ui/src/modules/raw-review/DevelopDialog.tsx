// Dialogo "Sviluppa dalla fotocamera" delle Varianti RAW (porting di VariantGenerator di FujiView):
// la fotocamera collegata sviluppa i RAF della sessione con i suoi banchi custom e scrive un JPEG
// per ricetta nel layout che la revisione legge. Lo sviluppo è un'attività: l'avanzamento sta in
// Attività, qui il riepilogo finale. Piano, spazio e tempo arrivano dal backend (`rawReviewDevelopPlan`).
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { Button, Dialog, FolderPicker, InlineAlert } from "../../components/ui";
import {
  commands,
  isTauri,
  type DevelopReport,
  type JobSnapshot,
  type RawReviewDevelopRequest,
} from "../../lib/ipc";
import { fmtBytes, fmtDuration, fmtInt } from "../../lib/format";
import { currentPhase } from "../../lib/jobs";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
import { CameraCheck } from "./CameraCheck";
import { effectiveSelection, startBlock, togglePreset } from "./develop";
import { codeView, errorView, pauseHint, type ErrorView } from "./errorView";
import { useCameraMonitor } from "./useCamera";

const DEBOUNCE_MS = 300;
const TERMINAL = ["completed", "completed_with_errors", "cancelled", "interrupted", "failed"];

type Scope = "all" | "selected";

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

/** Errore del backend: messaggio tradotto, azione suggerita e dettaglio tecnico. */
function ErrorBody({ view }: { view: ErrorView }) {
  return (
    <>
      {view.title && <p>{view.title}</p>}
      {view.hint && <p>{view.hint}</p>}
      <p className="muted dv-small">{view.detail}</p>
    </>
  );
}

function useErrorView() {
  const { t, i18n } = useTranslation();
  return (e: unknown) => errorView(e, t, (k) => i18n.exists(k));
}

export function DevelopDialog({
  open,
  onOpenChange,
  sessionId,
  selectedKeys,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string;
  /** Chiavi degli scatti selezionati nell'elenco della sessione. */
  selectedKeys: string[];
}) {
  const { t } = useTranslation();
  const viewOf = useErrorView();
  const [jobId, setJobId] = useState<number | null>(null);
  const job: JobSnapshot | undefined = useLive((s) => (jobId === null ? undefined : s.jobs[jobId]));
  const setup = open && jobId === null;
  const monitor = useCameraMonitor(open && (jobId === null || job?.state === "paused"));
  const { status, diagnostics, checking, recheck } = monitor;

  // Alla comparsa del dialogo si fa subito la verifica completa della fotocamera.
  useEffect(() => {
    if (setup) void recheck();
    // Solo all'apertura: "Ripeti verifica" la rifà quando serve.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setup]);

  const [touched, setTouched] = useState<string[] | null>(null);
  const [scopeChoice, setScopeChoice] = useState<Scope | null>(null);
  const [outputDir, setOutputDir] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [startError, setStartError] = useState<ErrorView | null>(null);

  const cameraReady = status?.available === true && status.state === "ready";
  const presetsQuery = useQuery({
    queryKey: ["raw-review-camera-presets", cameraReady],
    queryFn: () => commands.rawReviewCameraPresets(),
    enabled: isTauri() && setup && cameraReady,
    retry: false,
    staleTime: 0,
  });
  const presets = useMemo(() => presetsQuery.data?.presets ?? [], [presetsQuery.data]);
  const selected = effectiveSelection(presets, presetsQuery.data?.selectedIds ?? [], touched);

  const scope: Scope = scopeChoice ?? (selectedKeys.length > 0 ? "selected" : "all");
  const effectiveScope: Scope = scope === "selected" && selectedKeys.length === 0 ? "all" : scope;
  const shotKeys = effectiveScope === "selected" ? selectedKeys : null;

  const request: RawReviewDevelopRequest = useMemo(
    () => ({
      sessionId,
      shotKeys,
      presetIds: selected,
      outputDir: outputDir.trim() || null,
      overwrite,
    }),
    // `selected` cambia identità a ogni render: la sua chiave è il contenuto.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [sessionId, shotKeys, selected.join("|"), outputDir, overwrite],
  );
  const debounced = useDebounced(request, DEBOUNCE_MS);
  const settled = debounced === request || JSON.stringify(debounced) === JSON.stringify(request);
  const planEnabled =
    isTauri() && setup && cameraReady && presets.length > 0 && selected.length > 0;
  const planQuery = useQuery({
    queryKey: ["raw-review-develop-plan", debounced],
    queryFn: () => commands.rawReviewDevelopPlan(debounced),
    enabled: planEnabled && (shotKeys === null || shotKeys.length > 0),
    placeholderData: keepPreviousData,
    retry: false,
    // I file della cartella possono cambiare fuori dall'app.
    staleTime: 0,
  });
  const plan = planQuery.data;
  const planning = planEnabled && (!settled || planQuery.isFetching);

  const block = startBlock({
    status,
    diagnostics,
    checking,
    presetCount: selected.length,
    shotCount: shotKeys === null ? null : shotKeys.length,
    plan,
    planning,
    planFailed: planQuery.isError,
  });

  const start = async () => {
    setStartError(null);
    try {
      setJobId(await commands.startRawReviewDevelop(request));
    } catch (e) {
      setStartError(viewOf(e));
    }
  };

  const again = () => {
    setJobId(null);
    setStartError(null);
    setTouched(null);
  };

  const defaultOutput = plan?.outputDir ?? "";

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("rawDevelop.title")}
      description={jobId === null ? t("rawDevelop.intro") : undefined}
      width={720}
      footer={
        jobId === null ? (
          <>
            <Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>
            <Button
              variant="primary"
              disabledReason={block ? t(`rawDevelop.blocks.${block}`) : undefined}
              onClick={start}
              data-testid="raw-develop-start"
            >
              {t("rawDevelop.start")}
            </Button>
          </>
        ) : undefined
      }
    >
      {jobId !== null ? (
        <Progress
          jobId={jobId}
          job={job}
          monitor={monitor}
          onClose={() => onOpenChange(false)}
          onAgain={again}
        />
      ) : (
        <div className="dv-form">
          <section className="dv-section" aria-labelledby="dv-camera">
            <h3 id="dv-camera">{t("rawDevelop.camera")}</h3>
            <CameraCheck monitor={monitor} />
          </section>

          <fieldset className="dv-section dv-presets">
            <legend>{t("rawDevelop.presets")}</legend>
            <p className="muted dv-small">{t("rawDevelop.presetsHint")}</p>
            {!cameraReady && <p className="muted">{t("rawDevelop.presetsWaiting")}</p>}
            {presetsQuery.isError && (
              <InlineAlert tone="danger" title={t("rawDevelop.presetsFailed")}>
                <ErrorBody view={viewOf(presetsQuery.error)} />
              </InlineAlert>
            )}
            <ul className="dv-preset-list">
              {presets.map((p) => {
                const hintId = `dv-preset-${p.id}`;
                return (
                  <li key={p.id} className="dv-preset">
                    <label className="dv-check">
                      <input
                        type="checkbox"
                        checked={selected.includes(p.id)}
                        aria-describedby={hintId}
                        onChange={() => setTouched(togglePreset(selected, p.id))}
                      />
                      <strong>{p.id === "asshot" ? t("rawDevelop.asShot") : p.folderName}</strong>
                    </label>
                    <span id={hintId} className="dv-preset-info">
                      {p.id === "asshot" && (
                        <span className="muted dv-small">{t("rawDevelop.asShotHelp")}</span>
                      )}
                      {p.summary && <span className="muted dv-small">{p.summary}</span>}
                      {p.unappliedProperties.length > 0 && (
                        <span className="dv-warn dv-small">
                          {t("rawDevelop.unapplied", { list: p.unappliedProperties.join(", ") })}
                        </span>
                      )}
                    </span>
                  </li>
                );
              })}
            </ul>
            {presets.some((p) => p.unappliedProperties.length > 0) && (
              <p className="muted dv-small">{t("rawDevelop.unappliedHelp")}</p>
            )}
          </fieldset>

          <fieldset className="dv-section">
            <legend>{t("rawDevelop.scope.legend")}</legend>
            <label className="dv-check">
              <input
                type="radio"
                name="dv-scope"
                checked={effectiveScope === "all"}
                onChange={() => setScopeChoice("all")}
              />
              <span>{t("rawDevelop.scope.all")}</span>
            </label>
            <label className="dv-check">
              <input
                type="radio"
                name="dv-scope"
                checked={effectiveScope === "selected"}
                disabled={selectedKeys.length === 0}
                onChange={() => setScopeChoice("selected")}
              />
              <span>{t("rawDevelop.scope.selected", { count: selectedKeys.length })}</span>
            </label>
            {selectedKeys.length === 0 && (
              <p className="muted dv-small">{t("rawDevelop.scope.selectHint")}</p>
            )}
          </fieldset>

          <section className="dv-section" aria-labelledby="dv-output">
            <h3 id="dv-output">{t("rawDevelop.output.title")}</h3>
            <FolderPicker
              label={t("rawDevelop.output.label")}
              value={outputDir || defaultOutput}
              onChange={(p) => setOutputDir(p === defaultOutput ? "" : p)}
              hint={t("rawDevelop.output.hint")}
            />
            {outputDir && (
              <div>
                <Button size="sm" variant="ghost" onClick={() => setOutputDir("")}>
                  {t("rawDevelop.output.useDefault")}
                </Button>
              </div>
            )}
            <label className="dv-check">
              <input
                type="checkbox"
                checked={overwrite}
                onChange={(e) => setOverwrite(e.target.checked)}
              />
              <span>
                {t("rawDevelop.overwrite")}
                <br />
                <span className="muted dv-small">
                  {overwrite ? t("rawDevelop.overwriteOn") : t("rawDevelop.overwriteOff")}
                </span>
              </span>
            </label>
          </section>

          <PlanSummary
            planning={planning}
            enabled={planEnabled}
            error={planQuery.isError ? viewOf(planQuery.error) : null}
            plan={plan}
          />
          {startError && (
            <InlineAlert tone="danger" title={t("rawDevelop.startFailed")}>
              <ErrorBody view={startError} />
            </InlineAlert>
          )}
          <p className="muted dv-small">{t("rawDevelop.safety")}</p>
        </div>
      )}
    </Dialog>
  );
}

/** Riepilogo di quello che farà lo sviluppo, con l'avviso bloccante se manca lo spazio. */
function PlanSummary({
  plan,
  planning,
  enabled,
  error,
}: {
  plan: Awaited<ReturnType<typeof commands.rawReviewDevelopPlan>> | undefined;
  planning: boolean;
  enabled: boolean;
  error: ErrorView | null;
}) {
  const { t } = useTranslation();
  if (!enabled) return null;
  return (
    <section
      className="dv-section dv-summary"
      aria-labelledby="dv-summary"
      aria-busy={planning}
      data-testid="raw-develop-plan"
    >
      <h3 id="dv-summary">{t("rawDevelop.plan.title")}</h3>
      {error && (
        <InlineAlert tone="danger" title={t("rawDevelop.plan.failed")}>
          <ErrorBody view={error} />
        </InlineAlert>
      )}
      {planning && <p role="status">{t("rawDevelop.plan.computing")}</p>}
      {plan && (
        <div className={planning ? "dv-stale" : undefined}>
          <dl className="dv-facts">
            <dt>{t("rawDevelop.plan.raf")}</dt>
            <dd>{fmtInt(plan.rafCount)}</dd>
            <dt>{t("rawDevelop.plan.conversions")}</dt>
            <dd>{fmtInt(plan.totalConversions)}</dd>
            <dt>{t("rawDevelop.plan.alreadyDone")}</dt>
            <dd>{fmtInt(plan.alreadyDone)}</dd>
            <dt>{t("rawDevelop.plan.pending")}</dt>
            <dd>{fmtInt(plan.pendingConversions)}</dd>
            <dt>{t("rawDevelop.plan.space")}</dt>
            <dd>
              {plan.freeBytes === null
                ? t("rawDevelop.plan.spaceUnknown", { needed: fmtBytes(plan.estimatedBytes) })
                : t("rawDevelop.plan.spaceOf", {
                    needed: fmtBytes(plan.estimatedBytes),
                    free: fmtBytes(plan.freeBytes),
                  })}
            </dd>
            <dt>{t("rawDevelop.plan.time")}</dt>
            <dd>{fmtDuration(plan.estimatedSeconds)}</dd>
          </dl>
          <p className="muted dv-small">{t("rawDevelop.plan.into", { path: plan.outputDir })}</p>
          {plan.warnings.map((w) => (
            <p key={w} className="dv-warn">
              {w}
            </p>
          ))}
          {!plan.enoughSpace && (
            <InlineAlert tone="danger" title={t("rawDevelop.plan.noSpaceTitle")}>
              {t("rawDevelop.plan.noSpace", {
                needed: fmtBytes(plan.estimatedBytes),
                free: plan.freeBytes === null ? "?" : fmtBytes(plan.freeBytes),
                margin: fmtBytes(plan.safetyMarginBytes),
              })}
            </InlineAlert>
          )}
        </div>
      )}
    </section>
  );
}

/** Avanzamento (nelle Attività) e riepilogo finale dello sviluppo avviato. */
function Progress({
  jobId,
  job,
  monitor,
  onClose,
  onAgain,
}: {
  jobId: number;
  job: JobSnapshot | undefined;
  monitor: ReturnType<typeof useCameraMonitor>;
  onClose: () => void;
  onAgain: () => void;
}) {
  const { t, i18n } = useTranslation();
  const finished = job !== undefined && TERMINAL.includes(job.state);
  const [report, setReport] = useState<DevelopReport | null>(null);
  const [gaveUp, setGaveUp] = useState(false);
  const [controlError, setControlError] = useState("");
  const viewOf = useErrorView();

  // Il riepilogo esiste solo a lavoro finito: si chiede appena lo stato lo dice, con un po' di
  // pazienza se il backend lo scrive un istante dopo.
  useEffect(() => {
    if (!finished || report) return;
    let alive = true;
    let tries = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const ask = async () => {
      try {
        const r = await commands.rawReviewDevelopReport(jobId);
        if (!alive) return;
        if (r) {
          setReport(r);
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
    void ask();
    return () => {
      alive = false;
      if (timer) clearTimeout(timer);
    };
  }, [finished, report, jobId]);

  const control = async (run: () => Promise<unknown>) => {
    setControlError("");
    try {
      await run();
    } catch (e) {
      const view = viewOf(e);
      setControlError([view.title, view.hint, view.detail].filter(Boolean).join(" "));
    }
  };

  const openJobs = (
    <Button
      variant="ghost"
      onClick={() => {
        useLayout.getState().openModule({ module: "jobs", target: jobId });
        onClose();
      }}
    >
      {t("rawDevelop.openJobs")}
    </Button>
  );

  if (!report) {
    if (finished && gaveUp)
      return (
        <div className="dv-progress">
          <InlineAlert
            tone={job?.state === "failed" ? "danger" : "warn"}
            title={t("rawDevelop.noReport")}
            action={openJobs}
          >
            {job?.state === "failed" && job.error ? job.error : t("rawDevelop.noReportBody")}
          </InlineAlert>
          <div className="dv-final">
            <Button onClick={onClose}>{t("common.close")}</Button>
          </div>
        </div>
      );

    const paused = job?.state === "paused";
    const running = job !== undefined && !finished && !paused;
    const phase = job ? currentPhase(job) : undefined;
    const reason = job?.pauseReason;
    const cameraReason = reason?.code.startsWith("camera_");
    const cameraReady = monitor.status?.state === "ready";
    return (
      <div className="dv-progress">
        <p role="status">
          <strong>
            {paused ? t("rawDevelop.progress.paused") : t("rawDevelop.progress.running")}
          </strong>
        </p>
        {paused && (
          <InlineAlert
            tone="warn"
            title={
              reason
                ? t(`jobs.pauseReasons.${reason.code}`, {
                    defaultValue: t("rawDevelop.progress.pausedTitle"),
                  })
                : t("rawDevelop.progress.pausedTitle")
            }
          >
            <p>
              {pauseHint(reason?.code, t, (k) => i18n.exists(k)) ??
                t("rawDevelop.progress.pausedHelp")}
            </p>
            {reason?.detail && <p className="muted dv-small">{reason.detail}</p>}
          </InlineAlert>
        )}
        <progress
          className="dv-bar"
          aria-label={t("rawDevelop.progress.bar")}
          max={100}
          value={job?.overallPercent ?? undefined}
        />
        {phase && (
          <p className="muted">{t(`jobs.phases.${phase.name}`, { defaultValue: phase.name })}</p>
        )}
        {job?.overallPercent != null && (
          <p className="muted">
            {t("rawDevelop.progress.percent", { percent: Math.round(job.overallPercent) })}
          </p>
        )}
        {job && job.failed > 0 && (
          <p className="dv-warn">{t("rawDevelop.progress.failed", { count: job.failed })}</p>
        )}
        {paused && <CameraCheck compact monitor={monitor} />}
        {!paused && <p className="muted dv-small">{t("rawDevelop.progress.help")}</p>}
        {controlError && (
          <p role="alert" className="dv-warn">
            {controlError}
          </p>
        )}
        <div className="dv-final">
          {openJobs}
          {running && (
            <Button
              icon={<Icon.Pause size={16} />}
              onClick={() => void control(() => commands.pauseJob(jobId))}
            >
              {t("rawDevelop.progress.pause")}
            </Button>
          )}
          {paused && (
            <Button
              variant="primary"
              disabledReason={
                cameraReason && !cameraReady ? t("rawDevelop.progress.waitCamera") : undefined
              }
              onClick={() => void control(() => commands.resumeJob(jobId))}
            >
              {t("rawDevelop.progress.resume")}
            </Button>
          )}
          {(running || paused) && (
            <Button onClick={() => void control(() => commands.cancelJob(jobId))}>
              {t("rawDevelop.progress.cancel")}
            </Button>
          )}
          <Button onClick={onClose}>{t("common.close")}</Button>
        </div>
      </div>
    );
  }

  const reasonOf = (code: string, fallback: string) => {
    const key = `errors.${code.toLowerCase()}`;
    return i18n.exists(key) ? t(key) : fallback;
  };

  return (
    <div className="dv-progress" data-testid="raw-develop-report">
      <h3 className="dv-report-title" role="status">
        {report.stopped
          ? t("rawDevelop.report.stopped")
          : report.cancelled
            ? t("rawDevelop.report.cancelled")
            : report.failed > 0
              ? t("rawDevelop.report.withErrors")
              : t("rawDevelop.report.done")}
      </h3>
      {report.stopped && (
        <InlineAlert tone="danger" title={reasonOf(report.stopped.code, report.stopped.reason)}>
          <ErrorBody
            view={{
              ...codeView(report.stopped.code, report.stopped.reason, t, (k) => i18n.exists(k)),
              title: undefined,
            }}
          />
        </InlineAlert>
      )}
      <ul className="dv-report-list">
        <li>{t("rawDevelop.report.converted", { count: report.converted })}</li>
        {report.skipped > 0 && <li>{t("rawDevelop.report.skipped", { count: report.skipped })}</li>}
        {report.failed > 0 && <li>{t("rawDevelop.report.failed", { count: report.failed })}</li>}
        {report.converted + report.skipped + report.failed === 0 && !report.cancelled && (
          <li>{t("rawDevelop.report.nothing")}</li>
        )}
      </ul>
      <p className="muted dv-small">{t("rawDevelop.report.into", { path: report.outputDir })}</p>
      {report.converted > 0 && <p className="muted">{t("rawDevelop.report.panels")}</p>}
      {report.failures.length > 0 && (
        <div>
          <strong>{t("rawDevelop.report.failuresTitle")}</strong>
          <ul className="dv-failures" aria-label={t("rawDevelop.report.failuresTitle")}>
            {report.failures.map((f) => (
              <li key={f.raf + f.code + f.presets.join("|")}>
                <span>{f.raf}</span>
                <span className="muted"> — {reasonOf(f.code, f.reason)}</span>
                {f.presets.length > 0 && (
                  <small className="muted dv-block">
                    {t("rawDevelop.report.notDeveloped", { presets: f.presets.join(", ") })}
                  </small>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div className="dv-final">
        <Button variant="ghost" onClick={onAgain}>
          {t("rawDevelop.report.again")}
        </Button>
        <Button variant="primary" onClick={onClose}>
          {t("common.close")}
        </Button>
      </div>
    </div>
  );
}
