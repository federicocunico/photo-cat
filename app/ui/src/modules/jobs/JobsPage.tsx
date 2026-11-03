import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import {
  commands,
  isTauri,
  type JobRow,
  type JobSnapshot,
  type PhaseSnapshot,
  type ResourceSnapshot,
} from "../../lib/ipc";
import { fmtDateTime, fmtDec, fmtEta, fmtInt, mbToGb } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { activeJobs, useLive } from "../../store/live";
import { UpdateSummaryView } from "../import/manage";
import { useImportUpdates } from "../import/sourceActions";
import { FILE_WRITING_KINDS, jobObject, jobTitle, paramsOf } from "../../lib/jobs";
import { Button, ConfirmDialog, EmptyState, InlineAlert, PageHeader } from "../../components/ui";
import { runUndo, useUndo } from "../../store/undo";
import { notify } from "../../store/toasts";

const card = {
  background: "var(--pc-panel)",
  border: "1px solid var(--pc-border)",
  borderRadius: 8,
  padding: 20,
} as const;

function badgeStyle(state: string) {
  const map: Record<string, [string, string]> = {
    completed: ["var(--pc-ok-bg)", "var(--pc-ok)"],
    completed_with_errors: ["var(--pc-warn-bg)", "var(--pc-warn)"],
    queued: ["var(--pc-warn-bg)", "var(--pc-warn)"],
    paused: ["var(--pc-warn-bg)", "var(--pc-warn)"],
    running: ["var(--pc-accent-soft-bg)", "var(--pc-accent-soft-text)"],
  };
  const [bg, fg] = map[state] ?? ["var(--pc-danger-bg)", "var(--pc-danger)"];
  return { padding: "2px 8px", borderRadius: 4, background: bg, color: fg, fontSize: 12 } as const;
}

/** Conteggio della fase: la sorgente mostra i "trovati", le fasi a valle "elaborati / finora". */
function phaseCount(phase: PhaseSnapshot, isSource: boolean, t: TFunction) {
  const processed = phase.done + phase.failed;
  if (phase.totalKnown) return `${fmtInt(processed)} / ${fmtInt(phase.total)}`;
  if (isSource) return t("jobs.indeterminate", { n: fmtInt(phase.done) });
  // Totale provvisorio: lo spinner accanto indica che la scansione sta ancora contando.
  return `${fmtInt(processed)} / ${fmtInt(phase.total)}`;
}

export function PhaseRow({ phase, isSource }: { phase: PhaseSnapshot; isSource: boolean }) {
  const { t } = useTranslation();
  const pct = phase.percent;
  const done = phase.state === "done";
  return (
    <tr style={{ borderTop: "1px solid var(--pc-border)" }} data-testid={`phase-${phase.name}`}>
      <td style={{ padding: "10px 8px 10px 0" }}>
        {t(`jobs.phases.${phase.name}`, { defaultValue: phase.name })}
      </td>
      <td style={{ padding: "10px 8px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div
            className={`bar${pct == null && phase.state === "running" ? " indeterminate" : ""}`}
            style={{ flex: 1, height: 6 }}
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={pct ?? undefined}
            aria-label={t(`jobs.phases.${phase.name}`, { defaultValue: phase.name })}
          >
            <span
              style={{ width: `${pct ?? 0}%`, background: done ? "var(--pc-ok)" : undefined }}
            />
          </div>
          <span className="mono" style={{ width: 52, textAlign: "right" }}>
            {pct == null ? "—" : `${Math.floor(pct)}%`}
          </span>
        </div>
      </td>
      <td className="mono" style={{ padding: "10px 8px", whiteSpace: "nowrap" }}>
        {phaseCount(phase, isSource, t)}
        {!phase.totalKnown && !isSource && (
          <Icon.Spinner
            size={12}
            className="spin"
            data-testid="provisional-total"
            role="img"
            aria-hidden={false}
            aria-label={t("jobs.provisionalTotal")}
            style={{ marginLeft: 6, verticalAlign: -1, color: "var(--pc-text-soft)" }}
          />
        )}
      </td>
      <td className="mono" style={{ padding: "10px 8px", color: "var(--pc-text-soft)" }}>
        {phase.rate != null ? t("jobs.rate", { rate: fmtDec(phase.rate, 1) }) : "—"}
      </td>
      <td style={{ padding: "10px 8px", color: "var(--pc-text-soft)" }}>
        {phase.etaSecs != null ? fmtEta(phase.etaSecs) : t(`jobs.phaseState.${phase.state}`)}
      </td>
      <td
        style={{
          padding: "10px 8px",
          color: phase.failed > 0 ? "var(--pc-danger)" : "var(--pc-text-secondary)",
        }}
      >
        {phase.failed > 0 ? fmtInt(phase.failed) : "—"}
      </td>
    </tr>
  );
}

/** Titolo e oggetto di un'attività, dai parametri salvati nello storico. */
function useJobNames(rows: JobRow[] | undefined) {
  const { t } = useTranslation();
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: () => commands.listSources(),
    enabled: isTauri(),
  });
  const path = (id: number) => sources.data?.find((s) => s.source.id === id)?.source.path;
  return (id: number, kind: string) => {
    const params = paramsOf(rows, id);
    return { title: jobTitle(t, kind, params, path), object: jobObject(kind, params, path) };
  };
}

export function JobCard({
  job,
  title,
  object,
}: {
  job: JobSnapshot;
  title?: string;
  object?: string;
}) {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const name = title ?? jobTitle(t, job.kind, null);
  const [failure, setFailure] = useState<string | null>(null);
  const [confirmStop, setConfirmStop] = useState(false);
  // Gli errori di pausa, ripresa e interruzione compaiono nella scheda, dove sono stati avviati.
  const run = async (command: () => Promise<unknown>) => {
    setFailure(null);
    try {
      await command();
    } catch (error) {
      const detail = (error as { detail?: string } | null)?.detail;
      setFailure(detail ?? String(error));
    }
  };
  const stop = () => run(() => commands.cancelJob(job.id));
  const running = job.state === "running";
  const paused = job.state === "paused";
  const live = running || paused || job.state === "queued";
  // Finché la scansione non ha contato tutti i file la percentuale complessiva non è significativa.
  const counting = running && job.phases.some((p) => !p.totalKnown);
  return (
    <section
      style={{ ...card, display: "flex", flexDirection: "column", gap: 16 }}
      data-testid={`job-${job.id}`}
    >
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          <h2 style={{ margin: 0, fontSize: 17 }}>{name}</h2>
          <span className="muted" style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={badgeStyle(job.state)}>{t(`jobs.states.${job.state}`)}</span>
            {job.memoryPaused && (
              <span style={{ color: "var(--pc-warn)" }} data-testid={`memory-paused-${job.id}`}>
                {t("jobs.memoryPaused")}
              </span>
            )}
            {job.etaSecs != null && t("jobs.eta", { eta: fmtEta(job.etaSecs) })}
          </span>
        </div>
        {live && (
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            {running && (
              <Button
                icon={<Icon.Pause size={14} />}
                onClick={() => run(() => commands.pauseJob(job.id))}
              >
                {t("jobs.pause")}
              </Button>
            )}
            {paused && (
              <Button
                icon={<Icon.Play size={14} />}
                onClick={() => run(() => commands.resumeJob(job.id))}
              >
                {t("jobs.resume")}
              </Button>
            )}
            <Button
              variant="danger"
              icon={<Icon.Stop size={14} />}
              onClick={() => (FILE_WRITING_KINDS.has(job.kind) ? setConfirmStop(true) : stop())}
            >
              {t("jobs.cancel")}
            </Button>
            <ConfirmDialog
              open={confirmStop}
              onOpenChange={setConfirmStop}
              title={t("jobs.stopConfirmTitle", { title: name })}
              message={
                object ? t("jobs.stopConfirmBody", { object }) : t("jobs.stopConfirmBodyGeneric")
              }
              confirmLabel={t("jobs.cancel")}
              onConfirm={stop}
            />
          </div>
        )}
      </div>
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", fontWeight: 600 }}>
          <span>{t("jobs.overall")}</span>
          <span className="mono" data-testid={`overall-${job.id}`}>
            {counting ? "—" : `${Math.floor(job.overallPercent ?? 0)}%`}
          </span>
        </div>
        <div
          className={`bar${counting ? " indeterminate" : ""}`}
          style={{ height: 8, marginTop: 6, borderRadius: 4 }}
        >
          <span style={{ width: `${job.overallPercent ?? 0}%` }} />
        </div>
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", minWidth: 640 }}>
          <thead>
            <tr
              style={{
                color: "var(--pc-text-secondary)",
                textAlign: "left",
                fontSize: 12,
                textTransform: "uppercase",
                letterSpacing: 0.6,
              }}
            >
              <th style={{ padding: "8px 8px 8px 0" }}>{t("jobs.col.phase")}</th>
              <th style={{ padding: 8, width: "34%" }}>{t("jobs.col.progress")}</th>
              <th style={{ padding: 8 }}>{t("jobs.col.items")}</th>
              <th style={{ padding: 8 }}>{t("jobs.col.rate")}</th>
              <th style={{ padding: 8 }}>{t("jobs.col.eta")}</th>
              <th style={{ padding: 8 }}>{t("jobs.col.errors")}</th>
            </tr>
          </thead>
          <tbody>
            {job.phases.map((p, i) => (
              <PhaseRow key={p.name} phase={p} isSource={i === 0} />
            ))}
          </tbody>
        </table>
      </div>
      {job.failed > 0 && (
        <BusyButton
          type="button"
          className="btn btn-ghost"
          style={{ alignSelf: "flex-start", color: "var(--pc-danger)", border: 0, padding: 0 }}
          onClick={() =>
            openModule({ module: "settings", section: "diagnostics", target: { jobId: job.id } })
          }
        >
          {t("jobs.failed", { count: job.failed })} · {t("jobs.showErrors")}
        </BusyButton>
      )}
      {failure && (
        <InlineAlert
          tone="danger"
          title={t("jobs.commandFailed", { title: name })}
          onDismiss={() => setFailure(null)}
        >
          {failure}
        </InlineAlert>
      )}
      {job.error && <InlineAlert tone="danger">{job.error}</InlineAlert>}
      <JobUpdateSummary jobId={job.id} kind={job.kind} />
    </section>
  );
}

/** Riepilogo dell'aggiornamento nella riga di un import concluso. */
function JobUpdateSummary({ jobId, kind }: { jobId: number; kind: string }) {
  const updates = useImportUpdates();
  if (kind !== "import") return null;
  const s = (updates.data ?? []).find((u) => u.jobId === jobId);
  return s ? <UpdateSummaryView summary={s} /> : null;
}

function History({
  rows,
  liveIds,
  names,
}: {
  rows: JobRow[];
  liveIds: Set<number>;
  names: (id: number, kind: string) => { title: string };
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const visible = rows.filter((r) => !liveIds.has(r.id));
  if (visible.length === 0) return null;
  return (
    <section
      aria-labelledby="jobs-history"
      style={{ ...card, padding: "16px 20px", display: "flex", flexDirection: "column", gap: 10 }}
    >
      <h2 id="jobs-history" style={{ margin: 0, fontSize: 16 }}>
        {t("jobs.history")}
      </h2>
      {visible.map((r) => (
        <div
          key={r.id}
          style={{ display: "flex", flexWrap: "wrap", gap: 12, alignItems: "center" }}
        >
          <span style={badgeStyle(r.state)}>{t(`jobs.states.${r.state}`)}</span>
          <span>{names(r.id, r.kind).title}</span>
          <span className="muted" style={{ fontSize: 12 }}>
            {fmtDateTime(r.finishedAt ?? r.startedAt ?? r.createdAt)}
          </span>
          <JobUpdateSummary jobId={r.id} kind={r.kind} />
          {r.state === "interrupted" && (
            <BusyButton
              type="button"
              className="btn"
              style={{ marginLeft: "auto" }}
              onClick={async () => {
                await commands.resumeInterruptedJob(r.id);
                return await qc.invalidateQueries({ queryKey: ["job-history"] });
              }}
            >
              {t("jobs.resumeInterrupted")}
            </BusyButton>
          )}
        </div>
      ))}
    </section>
  );
}

export function ResourcesPanel({ res }: { res: ResourceSnapshot | null }) {
  const { t } = useTranslation();
  if (!res) return null;
  const { system, governor } = res;
  const usedPct = ((system.totalMb - system.availableMb) / Math.max(1, system.totalMb)) * 100;
  const appPct = (system.processMb / Math.max(1, system.totalMb)) * 100;
  return (
    <section
      style={{ ...card, display: "flex", flexDirection: "column", gap: 14 }}
      aria-label={t("jobs.resources")}
    >
      <h2 style={{ margin: 0, fontSize: 16 }}>{t("jobs.resources")}</h2>
      <div
        role="img"
        aria-label={`${t("jobs.systemRam")} ${mbToGb(system.totalMb)} GB`}
        style={{
          display: "flex",
          height: 14,
          borderRadius: 4,
          overflow: "hidden",
          background: "var(--pc-border)",
        }}
      >
        <div
          style={{
            width: `${Math.max(0, usedPct - appPct)}%`,
            background: "var(--pc-text-placeholder)",
          }}
        />
        <div style={{ width: `${appPct}%`, background: "var(--pc-accent)" }} />
      </div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
          gap: "6px 12px",
          fontSize: 13,
        }}
      >
        <span className="muted">{t("jobs.systemRam")}</span>
        <span>{mbToGb(system.totalMb)} GB</span>
        <span className="muted">{t("jobs.available")}</span>
        <span>{mbToGb(system.availableMb)} GB</span>
        <span className="muted">{t("jobs.appBudget")}</span>
        <span>{mbToGb(governor.budgetMb)} GB</span>
        <span className="muted">{t("jobs.inUse")}</span>
        <span>
          {t("jobs.processMem", { mb: fmtInt(system.processMb) })} ·{" "}
          {t("jobs.workMem", { mb: fmtInt(governor.inUseMb) })}
        </span>
        <span className="muted">{t("jobs.cores")}</span>
        <span>
          {system.logicalCores} / {system.physicalCores}
        </span>
        <span className="muted">{t("jobs.pressure")}</span>
        <span>{t(`jobs.pressureStates.${res.state}`)}</span>
      </div>
      <h3 className="section-title" style={{ marginTop: 4 }}>
        {t("jobs.residentModels")}
      </h3>
      {governor.residents.length === 0 && <span className="muted">{t("jobs.noModels")}</span>}
      {governor.residents.map(([name, mb]) => (
        <div key={name} style={{ display: "flex", justifyContent: "space-between" }}>
          <span>{name}</span>
          <span className="mono">{fmtInt(mb)} MB</span>
        </div>
      ))}
      {governor.aiPaused && <span style={{ color: "var(--pc-warn)" }}>{t("jobs.aiPaused")}</span>}
    </section>
  );
}

/**
 * Ultima operazione annullabile (ui-foundations, "Annullamento delle operazioni"). Il
 * backend tiene un registro LIFO: se l'app non ha registrato l'operazione (per esempio
 * dopo un riavvio), si mostra la descrizione del backend e si annulla con `undoBulk`.
 */
function UndoableOperation() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const registered = useUndo((s) => s.last);
  const backend = useQuery({
    queryKey: ["last-undo"],
    queryFn: () => commands.lastBulkUndo(),
    enabled: isTauri() && !registered,
    refetchInterval: 5000,
  });
  const label = registered?.label ?? backend.data;
  if (!label) return null;
  const undo = async () => {
    if (registered) {
      await runUndo();
    } else {
      await commands.undoBulk();
      notify({ key: "undo", message: t("common.undone", { label }) });
      await qc.invalidateQueries();
    }
    await qc.invalidateQueries({ queryKey: ["last-undo"] });
  };
  return (
    <section
      aria-labelledby="jobs-undoable"
      style={{ ...card, padding: "16px 20px", display: "flex", gap: 12, alignItems: "center" }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 2, flex: 1, minWidth: 0 }}>
        <h2 id="jobs-undoable" className="section-title" style={{ margin: 0 }}>
          {t("jobs.undoable")}
        </h2>
        <span>{label}</span>
      </div>
      <Button icon={<Icon.Undo size={14} />} onClick={undo}>
        {t("common.undo")}
      </Button>
    </section>
  );
}

export function JobsPage() {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const jobs = useLive((s) => s.jobs);
  const resources = useLive((s) => s.resources);
  const history = useQuery({
    queryKey: ["job-history"],
    queryFn: () => commands.listJobHistory(30),
    enabled: isTauri(),
    refetchInterval: 5000,
  });
  const live = activeJobs(jobs);
  const finished = Object.values(jobs)
    .filter((j) => !live.includes(j))
    .sort((a, b) => b.id - a.id);
  const shown = [...live, ...finished.slice(0, 3)];
  const names = useJobNames(history.data);
  return (
    <QueryView query={history} enabled={isTauri()} isEmpty={() => false}>
      {() => (
        <div
          style={{
            flex: 1,
            overflow: "auto",
            padding: "24px 32px",
            display: "flex",
            flexWrap: "wrap",
            gap: 24,
            alignContent: "flex-start",
          }}
        >
          <div
            style={{
              flex: "999 1 560px",
              minWidth: 0,
              display: "flex",
              flexDirection: "column",
              gap: 16,
            }}
          >
            <PageHeader
              title={t("jobs.title")}
              actions={
                import.meta.env.DEV &&
                isTauri() && (
                  <BusyButton
                    type="button"
                    className="btn"
                    onClick={() => commands.startDemoJob(3000, 400)}
                  >
                    {t("jobs.demo")}
                  </BusyButton>
                )
              }
            />
            {shown.length === 0 && (
              <EmptyState
                icon={<Icon.Jobs size={28} />}
                title={t("jobs.empty")}
                action={
                  <Button variant="primary" onClick={() => openModule({ module: "import" })}>
                    {t("jobs.emptyAction")}
                  </Button>
                }
              />
            )}
            {shown.map((j) => (
              <JobCard key={j.id} job={j} {...names(j.id, j.kind)} />
            ))}
            <UndoableOperation />
            <History
              rows={history.data ?? []}
              liveIds={new Set(shown.map((j) => j.id))}
              names={names}
            />
          </div>
          <aside
            style={{
              flex: "1 1 300px",
              minWidth: 0,
              display: "flex",
              flexDirection: "column",
              gap: 16,
            }}
          >
            <ResourcesPanel res={resources} />
          </aside>
        </div>
      )}
    </QueryView>
  );
}
