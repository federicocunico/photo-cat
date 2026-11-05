// Sezione AI > Risultati (ai-workspace D4, D6): distribuzione per gruppo, istogramma della
// confidenza di una voce con la soglia, copertura mensile. Gli intervalli aprono la Libreria
// o la sezione Pulizia.
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { BarChart, ColumnSeries, Histogram } from "../../components/charts";
import type { HistogramBin, HistogramRange } from "../../components/charts";
import { Button, InlineAlert } from "../../components/ui";
import { commands, isTauri, type LibraryFilter } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { emptyFilter } from "../../store/library";
import { useLabelName, useTaxonomy } from "../library/labelName";
import { openCleanupWith, openLibraryWith } from "./results/navigate";

const BINS = 20;
const ALL = "all";
const f2 = (n: number) => n.toFixed(2);

export function ResultsSection() {
  const { t } = useTranslation();
  const name = useLabelName();
  const tax = useTaxonomy().data;
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const [scope, setScope] = useState<number | null>(null);
  const [groupSel, setGroupSel] = useState<string | null>(null);
  const [valueSel, setValueSel] = useState<string | null>(null);
  const [range, setRange] = useState<HistogramRange | null>(null);

  const filter: LibraryFilter = useMemo(() => ({ ...emptyFilter(), catalogId: scope }), [scope]);
  const groups = useMemo(() => tax?.groups ?? [], [tax]);
  const group = groupSel && groups.some((g) => g.id === groupSel) ? groupSel : (groups[0]?.id ?? null);

  const coverage = useQuery({
    queryKey: ["ai-results", "coverage", scope],
    queryFn: () => commands.aiMonthCoverage(filter),
    enabled: isTauri(),
  });
  const dist = useQuery({
    queryKey: ["ai-results", "dist", scope, group],
    queryFn: () =>
      commands.libraryFacetValues(filter, { kind: "label", group: group! }, null, "count", 0, 200),
    enabled: isTauri() && group !== null,
  });
  const distValues = useMemo(() => dist.data?.values ?? [], [dist.data]);
  const value =
    valueSel && distValues.some((v) => v.value === valueSel) ? valueSel : (distValues[0]?.value ?? null);
  const hist = useQuery({
    queryKey: ["ai-results", "hist", scope, group, value],
    queryFn: () => commands.aiLabelHistogram(filter, group!, value!, BINS),
    enabled: isTauri() && group !== null && value !== null,
  });

  // L'intervallo vale per una sola voce e un solo ambito.
  useEffect(() => setRange(null), [scope, group, value]);

  const bins: HistogramBin[] = useMemo(
    () => (hist.data ?? []).map((b) => ({ from: b.from ?? 0, to: b.to ?? 1, count: b.count })),
    [hist.data],
  );
  const threshold = useMemo(() => {
    const g = groups.find((x) => x.id === group);
    const c = g?.categories.find((x) => x.id === value);
    return c?.threshold ?? g?.threshold ?? undefined;
  }, [groups, group, value]);

  const scopes = (catalogs.data ?? []).filter((c) => !c.catalog.system);
  const months = coverage.data ?? [];
  const analyzedTotal = months.reduce((n, m) => n + m.analyzed, 0);

  const header = (
    <>
      <h2 id="ai-section-title">{t("ai.sections.results")}</h2>
      <p className="muted">{t("aiResults.intro")}</p>
    </>
  );

  const scopeSelect = (
    <label className="ai-results-field">
      <span>{t("aiResults.scope")}</span>
      <select
        value={scope === null ? ALL : String(scope)}
        onChange={(e) => setScope(e.target.value === ALL ? null : Number(e.target.value))}
      >
        <option value={ALL}>{t("aiResults.scopeAll")}</option>
        {scopes.map((c) => (
          <option key={c.catalog.id} value={String(c.catalog.id)}>
            {c.catalog.name}
          </option>
        ))}
      </select>
    </label>
  );

  if (coverage.isError || dist.isError)
    return (
      <section className="ai-section" aria-labelledby="ai-section-title">
        {header}
        {scopeSelect}
        <InlineAlert tone="danger">{t("aiResults.error")}</InlineAlert>
      </section>
    );

  if (coverage.data === undefined)
    return (
      <section className="ai-section" aria-labelledby="ai-section-title" aria-busy="true">
        {header}
        {scopeSelect}
        <p className="muted" role="status">
          {t("aiResults.loading")}
        </p>
      </section>
    );

  if (analyzedTotal === 0)
    return (
      <section className="ai-section" aria-labelledby="ai-section-title">
        {header}
        {scopeSelect}
        <div className="ai-results-empty" data-testid="results-empty">
          <p>{t("aiResults.empty")}</p>
          <Button
            variant="primary"
            onClick={() => useLayout.getState().openModule({ module: "ai", section: "reprocess" })}
          >
            {t("aiResults.emptyLink")}
          </Button>
        </div>
      </section>
    );

  const noDate = months.find((m) => m.month === null);
  const columns = months
    .filter((m) => m.month !== null)
    .sort((a, b) => (a.month! < b.month! ? -1 : 1))
    .map((m) => ({ key: m.month!, label: m.month!, total: m.total, analyzed: m.analyzed }));
  const valueName = group !== null && value !== null ? name.label(group, value) : "";

  return (
    <section className="ai-section" aria-labelledby="ai-section-title">
      {header}
      <div className="ai-results-controls">
        {scopeSelect}
        <label className="ai-results-field">
          <span>{t("aiResults.group")}</span>
          <select
            value={group ?? ""}
            onChange={(e) => {
              setGroupSel(e.target.value);
              setValueSel(null);
            }}
          >
            {groups.map((g) => (
              <option key={g.id} value={g.id}>
                {name.group(g.id)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <h3>{t("aiResults.distribution")}</h3>
      <p className="muted">{t("aiResults.distributionHint")}</p>
      {group !== null &&
        (dist.data === undefined ? (
          <p className="muted" role="status">
            {t("aiResults.loading")}
          </p>
        ) : (
          <BarChart
            items={distValues.map((v) => ({ key: v.value, label: name.label(group, v.value), value: v.count }))}
            emptyText={t("aiResults.distributionEmpty")}
            ariaLabel={t("aiResults.distributionLabel", { group: name.group(group) })}
            onSelect={(key) => {
              setValueSel(key);
              openLibraryWith(scope, { labels: [{ group, values: [key] }] });
            }}
          />
        ))}

      <h3>{value !== null ? t("aiResults.histogram", { value: valueName }) : t("aiResults.value")}</h3>
      {value === null || group === null ? (
        <p className="muted">{t("aiResults.pickValue")}</p>
      ) : (
        <>
          <label className="ai-results-field">
            <span>{t("aiResults.value")}</span>
            <select value={value} onChange={(e) => setValueSel(e.target.value)}>
              {distValues.map((v) => (
                <option key={v.value} value={v.value}>
                  {name.label(group, v.value)}
                </option>
              ))}
            </select>
          </label>
          {hist.data === undefined ? (
            <p className="muted" role="status">
              {t("aiResults.loading")}
            </p>
          ) : (
            <Histogram
              bins={bins}
              threshold={threshold}
              thresholdLabel={threshold !== undefined ? t("aiResults.threshold", { value: f2(threshold) }) : undefined}
              range={range}
              onRangeChange={setRange}
              emptyText={t("aiResults.histogramEmpty")}
              ariaLabel={t("aiResults.histogramLabel", { value: valueName })}
              hint={t("aiResults.histogramHint")}
              formatBin={(b) => t("aiResults.bin", { from: f2(b.from), to: f2(b.to), count: b.count })}
              formatRange={(r) => t("aiResults.range", { min: f2(r[0]), max: f2(r[1]) })}
            />
          )}
          {range && (
            <div
              className="ai-results-actions"
              role="group"
              aria-label={t("aiResults.range", { min: f2(range[0]), max: f2(range[1]) })}
            >
              <Button
                variant="primary"
                onClick={() =>
                  openLibraryWith(scope, { labelScore: { grp: group, value, min: range[0], max: range[1] } })
                }
              >
                {t("aiResults.showInLibrary")}
              </Button>
              <Button onClick={() => openCleanupWith(scope, group, value, range[1])}>
                {t("aiResults.cleanup")}
              </Button>
              <Button variant="ghost" onClick={() => setRange(null)}>
                {t("aiResults.clearRange")}
              </Button>
            </div>
          )}
        </>
      )}

      <h3>{t("aiResults.coverage")}</h3>
      <ColumnSeries
        items={columns}
        noDate={
          noDate
            ? { key: "none", label: t("aiResults.noDate"), total: noDate.total, analyzed: noDate.analyzed }
            : undefined
        }
        emptyText={t("aiResults.coverageEmpty")}
        ariaLabel={t("aiResults.coverageLabel")}
        totalLabel={t("aiResults.total")}
        analyzedLabel={t("aiResults.analyzed")}
      />
    </section>
  );
}
