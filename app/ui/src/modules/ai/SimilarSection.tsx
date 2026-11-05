import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { BarChart } from "../../components/charts";
import { Button, InlineAlert } from "../../components/ui";
import { fmtInt } from "../../lib/format";
import { commands } from "../../lib/ipc";
import type { SizeBucket } from "../../bindings";
import { useLibrary } from "../../store/library";
import { notify } from "../../store/toasts";
import { openLibraryWith } from "./results/navigate";
import { SIMILAR_STATS_KEY, useActiveSimilarJob, useSimilarStats } from "./similar/useSimilarStats";

const ORDER: SizeBucket[] = ["two", "three_five", "six_ten", "over10"];

/** Libreria con la vista raggruppata per simili, filtro nuovo e selezione azzerata. */
function openLibrary(patch: { similarSize?: SizeBucket; similarOnly?: boolean }) {
  useLibrary.getState().setGroupSimilar(true);
  openLibraryWith(null, patch);
}

function Kpi({ label, value, id }: { label: string; value: number; id: string }) {
  return (
    <div className="ai-kpi" data-testid={`kpi-${id}`}>
      <span className="ai-kpi-value">{fmtInt(value)}</span>
      <span className="ai-kpi-label">{label}</span>
    </div>
  );
}

export function SimilarSection() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const query = useSimilarStats();
  const stats = query.data;
  const job = useActiveSimilarJob();
  const [starting, setStarting] = useState(false);

  const start = async () => {
    setStarting(true);
    try {
      await commands.startSimilarAnalysis();
      void qc.invalidateQueries({ queryKey: SIMILAR_STATS_KEY });
    } catch {
      notify({ tone: "error", message: t("aiSimilar.startFailed") });
    } finally {
      setStarting(false);
    }
  };

  const chartItems = ORDER.map((k) => ({
    key: k,
    label: t(`aiSimilar.bucket.${k}`),
    value: stats?.sizeBuckets.find((b) => b.bucket === k)?.groups ?? 0,
  }));
  const pending = stats?.pending ?? 0;
  const percent = job?.overallPercent;

  return (
    <section className="ai-section" aria-labelledby="ai-section-title">
      <h2 id="ai-section-title">{t("ai.sections.similar")}</h2>
      <p className="muted">{t("aiSimilar.intro")}</p>
      {query.isLoading && (
        <p className="muted" role="status">
          {t("aiSimilar.loading")}
        </p>
      )}
      {query.isError && (
        <InlineAlert
          tone="danger"
          action={<Button onClick={() => void query.refetch()}>{t("common.retry")}</Button>}
        >
          {t("aiSimilar.error")}
        </InlineAlert>
      )}
      {stats && (
        <>
          <div className="ai-kpis" role="group" aria-label={t("aiSimilar.kpis")}>
            <Kpi id="groups" label={t("aiSimilar.groups")} value={stats.groups} />
            <Kpi id="photos" label={t("aiSimilar.photos")} value={stats.groupedPhotos} />
            <Kpi id="pending" label={t("aiSimilar.pending")} value={pending} />
          </div>
          {(pending > 0 || job) && (
            <div className="ai-similar-actions">
              <Button variant="primary" disabled={!!job || starting} onClick={() => void start()}>
                {t("aiSimilar.start")}
              </Button>
              <span className="muted" role="status">
                {job
                  ? percent != null
                    ? t("aiSimilar.running", { percent: Math.round(percent) })
                    : t("aiSimilar.runningNoPercent")
                  : t("aiSimilar.pendingHint", { count: pending })}
              </span>
            </div>
          )}
          {stats.groups === 0 ? (
            <div className="ai-similar-empty">
              <h3>{t("aiSimilar.emptyTitle")}</h3>
              <p className="muted">{t("aiSimilar.emptyBody")}</p>
            </div>
          ) : (
            <>
              <h3>{t("aiSimilar.chartTitle")}</h3>
              <p className="muted">{t("aiSimilar.chartHint")}</p>
              <BarChart
                items={chartItems}
                sorted={false}
                ariaLabel={t("aiSimilar.chartLabel")}
                emptyText={t("aiSimilar.emptyChart")}
                barLabel={(i) => t("aiSimilar.barLabel", { label: i.label, count: i.value })}
                onSelect={(k) => openLibrary({ similarSize: k as SizeBucket })}
              />
              <Button variant="ghost" onClick={() => openLibrary({ similarOnly: true })}>
                {t("aiSimilar.showSimilar")}
              </Button>
            </>
          )}
        </>
      )}
    </section>
  );
}
