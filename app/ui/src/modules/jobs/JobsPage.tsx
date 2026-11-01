import { useQuery, useQueryClient } from "@tanstack/react-query";
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

export function JobCard({ job }: { job: JobSnapshot }) {
  const { t } = useTranslation();
  const openDiagnostics = useLayout((s) => s.openDiagnostics);
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
          <strong style={{ fontSize: 17 }}>
            #{job.id} · {job.kind}
          </strong>
          <span className="muted" style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={badgeStyle(job.state)}>{t(`jobs.states.${job.state}`)}</span>
            {job.etaSecs != null && t("jobs.eta", { eta: fmtEta(job.etaSecs) })}
          </span>
        </div>
        {live && (
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            {running && (
              <button type="button" className="btn" onClick={() => commands.pauseJob(job.id)}>
                <Icon.Pause size={14} />
                {t("jobs.pause")}
              </button>
            )}
            {paused && (
              <button type="button" className="btn" onClick={() => commands.resumeJob(job.id)}>
                <Icon.Play size={14} />
                {t("jobs.resume")}
              </button>
            )}
            <button
              type="button"
              className="btn btn-danger"
              onClick={() => commands.cancelJob(job.id)}
            >
              <Icon.Stop size={14} />
              {t("jobs.cancel")}
            </button>
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
        <button
          type="button"
          className="btn btn-ghost"
          style={{ alignSelf: "flex-start", color: "var(--pc-danger)", border: 0, padding: 0 }}
          onClick={() => openDiagnostics({ jobId: job.id })}
        >
          {t("jobs.failed", { count: job.failed })} · {t("jobs.showErrors")}
        </button>
      )}
      {job.error && <div style={{ color: "var(--pc-danger)" }}>{job.error}</div>}
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

function History({ rows, liveIds }: { rows: JobRow[]; liveIds: Set<number> }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const visible = rows.filter((r) => !liveIds.has(r.id));
  if (visible.length === 0) return null;
  return (
    <section
      style={{ ...card, padding: "16px 20px", display: "flex", flexDirection: "column", gap: 10 }}
    >
      {visible.map((r) => (
        <div
          key={r.id}
          style={{ display: "flex", flexWrap: "wrap", gap: 12, alignItems: "center" }}
        >
          <span style={badgeStyle(r.state)}>{t(`jobs.states.${r.state}`)}</span>
          <span>
            #{r.id} · {r.kind}
          </span>
          <span className="muted mono" style={{ fontSize: 12 }}>
            {fmtDateTime(r.createdAt)}
          </span>
          <JobUpdateSummary jobId={r.id} kind={r.kind} />
          {r.state === "interrupted" && (
            <button
              type="button"
              className="btn"
              style={{ marginLeft: "auto" }}
              onClick={async () => {
                await commands.resumeInterruptedJob(r.id);
                await qc.invalidateQueries({ queryKey: ["job-history"] });
              }}
            >
              {t("jobs.resumeInterrupted")}
            </button>
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

export function JobsPage() {
  const { t } = useTranslation();
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
  return (
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
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("jobs.title")}</h1>
          {import.meta.env.DEV && isTauri() && (
            <button
              type="button"
              className="btn"
              style={{ marginLeft: "auto" }}
              onClick={() => commands.startDemoJob(3000, 400)}
            >
              {t("jobs.demo")}
            </button>
          )}
        </div>
        {shown.length === 0 && <p className="muted">{t("jobs.empty")}</p>}
        {shown.map((j) => (
          <JobCard key={j.id} job={j} />
        ))}
        <History rows={history.data ?? []} liveIds={new Set(shown.map((j) => j.id))} />
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
  );
}
