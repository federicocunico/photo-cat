import { useTranslation } from "react-i18next";
import { StackedBar } from "../../../components/charts";
import type { Segment } from "../../../components/charts";
import type { Stage, StageCoverage } from "../../../lib/ipc";
import { fmtInt } from "../../../lib/format";

export type SegmentKey = "current" | "stale" | "queued" | "never";
const ORDER: SegmentKey[] = ["current", "stale", "queued", "never"];

export function StageRow({
  cov,
  onSelect,
}: {
  cov: StageCoverage;
  onSelect: (stage: Stage, key: SegmentKey) => void;
}) {
  const { t } = useTranslation();
  const name = t(`aiOverview.stage.${cov.stage}`);
  const segments: Segment[] = ORDER.map((key) => ({
    key,
    label: t(`aiOverview.state.${key}`),
    value: cov[key],
    tone: key,
  }));
  return (
    <li className="ai-overview-stage" data-testid={`stage-${cov.stage}`}>
      <div className="ai-overview-stage-head">
        <h4>{name}</h4>
        <span className="muted">{t("aiOverview.total", { n: fmtInt(cov.total) })}</span>
      </div>
      <StackedBar
        segments={segments}
        onSelect={(key) => onSelect(cov.stage, key as SegmentKey)}
        emptyText={t("aiOverview.empty")}
        ariaLabel={t("aiOverview.barLabel", { name })}
        formatValue={fmtInt}
      />
    </li>
  );
}

