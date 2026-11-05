import { useTranslation } from "react-i18next";
import { Button, InlineAlert } from "../../components/ui";
import { fmtInt, mbToGb } from "../../lib/format";
import type { JobSnapshot, Stage } from "../../lib/ipc";
import { currentPhase, jobTitle } from "../../lib/jobs";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
import { openLibraryWith } from "./results/navigate";
import { StageRow } from "./overview/StageRow";
import type { SegmentKey } from "./overview/StageRow";
import { useActiveAiJobs, useAiOverview } from "./overview/useAiOverview";

function JobLine({ job }: { job: JobSnapshot }) {
  const { t } = useTranslation();
  const phase = currentPhase(job);
  const pct = Math.floor(phase?.percent ?? job.overallPercent ?? 0);
  return (
    <li className="ai-overview-job" data-testid={`ai-job-${job.id}`}>
      <button
        type="button"
        onClick={() => useLayout.getState().openModule({ module: "jobs", target: job.id })}
      >
        {jobTitle(t, job.kind, null)}
      </button>
      {phase && (
        <span className="muted">
          {t(`jobs.phases.${phase.name}`, { defaultValue: phase.name })}
        </span>
      )}
      <span className="mono">{pct}%</span>
    </li>
  );
}

/** Panoramica: copertura delle analisi per fase, attività AI in corso e risorse. */
export function OverviewSection() {
  const { t } = useTranslation();
  const { data, isLoading, isError, refetch } = useAiOverview();
  const jobs = useActiveAiJobs();
  const resources = useLive((s) => s.resources);

  // "In coda": le foto sono già in un'attività, quindi si apre quella (o l'elenco).
  const select = (stage: Stage, key: SegmentKey) => {
    if (key === "queued") {
      const target = jobs[0]?.id;
      useLayout
        .getState()
        .openModule(target != null ? { module: "jobs", target } : { module: "jobs" });
      return;
    }
    openLibraryWith(null, { analysis: { stage, state: key } });
  };

  return (
    <section className="ai-section" aria-labelledby="ai-section-title">
      <h2 id="ai-section-title">{t("ai.sections.overview")}</h2>
      <p className="muted">{t("aiOverview.hint")}</p>

      <h3>{t("aiOverview.coverage")}</h3>
      {isLoading && (
        <p className="muted" role="status">
          {t("aiOverview.loading")}
        </p>
      )}
      {isError && (
        <InlineAlert
          tone="danger"
          action={<Button onClick={() => void refetch()}>{t("aiOverview.retry")}</Button>}
        >
          {t("aiOverview.error")}
        </InlineAlert>
      )}
      {data && (
        <ul className="ai-overview-stages">
          {data.stages.map((cov) => (
            <StageRow key={cov.stage} cov={cov} onSelect={select} />
          ))}
        </ul>
      )}

      <h3>{t("aiOverview.activity")}</h3>
      {jobs.length === 0 ? (
        <p className="muted">{t("aiOverview.noJobs")}</p>
      ) : (
        <ul className="ai-overview-jobs">
          {jobs.map((j) => (
            <JobLine key={j.id} job={j} />
          ))}
        </ul>
      )}
      <div>
        <Button variant="ghost" onClick={() => useLayout.getState().openModule({ module: "jobs" })}>
          {t("aiOverview.openJobs")}
        </Button>
      </div>

      <h3>{t("aiOverview.resources")}</h3>
      {resources ? (
        <dl className="ai-overview-resources">
          <dt>{t("aiOverview.memory")}</dt>
          <dd className="mono">
            {t("aiOverview.memoryValue", {
              used: mbToGb(resources.governor.inUseMb),
              budget: mbToGb(resources.governor.budgetMb),
            })}
          </dd>
          <dt>{t("aiOverview.models")}</dt>
          <dd>
            {resources.governor.residents.length === 0
              ? t("aiOverview.noModels")
              : resources.governor.residents
                  .map(([name, mb]) => `${name} (${fmtInt(mb)} MB)`)
                  .join(", ")}
          </dd>
        </dl>
      ) : (
        <p className="muted">{t("aiOverview.noResources")}</p>
      )}
    </section>
  );
}
