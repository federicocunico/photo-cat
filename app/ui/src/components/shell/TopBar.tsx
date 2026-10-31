import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { activeJobs, useLive } from "../../store/live";
import { useLayout } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { fmtDec, fmtInt, mbToGb } from "../../lib/format";
import { currentPhase } from "../../lib/jobs";

/** Ricerca globale: filtra la Libreria per testo (nome, percorso, luogo, persone, tag). */
function GlobalSearch() {
  const { t } = useTranslation();
  const text = useLibrary((s) => s.filter.text ?? "");
  const setFilter = useLibrary((s) => s.setFilter);
  const setModule = useLayout((s) => s.setModule);
  const [value, setValue] = useState(text);
  useEffect(() => setValue(text), [text]);
  useEffect(() => {
    if (value === text) return;
    const h = setTimeout(() => {
      setFilter({ text: value.trim() ? value : null });
      if (value.trim()) setModule("library");
    }, 250);
    return () => clearTimeout(h);
  }, [value, text, setFilter, setModule]);
  return (
    <input
      type="search"
      aria-label={t("app.searchLabel")}
      placeholder={t("app.search")}
      value={value}
      onChange={(e) => setValue(e.target.value)}
      style={{
        flex: 1,
        background: "transparent",
        border: 0,
        color: "var(--pc-text)",
        outline: "none",
      }}
    />
  );
}

export function JobPill() {
  const { t } = useTranslation();
  const jobs = useLive((s) => s.jobs);
  const setModule = useLayout((s) => s.setModule);
  const job = activeJobs(jobs)[0];
  const percent = job ? Math.floor(job.overallPercent ?? 0) : 0;
  const phase = job ? currentPhase(job) : undefined;
  const phaseName = phase ? t(`jobs.phases.${phase.name}`, { defaultValue: phase.name }) : "";
  const label = !job
    ? t("shell.noJobs")
    : phase && !phase.totalKnown
      ? `${phaseName} · ${t("jobs.indeterminate", { n: fmtInt(phase.done) })}`
      : t("shell.jobPill", { phase: phaseName, percent });
  return (
    <button type="button" className="pill" onClick={() => setModule("jobs")} data-testid="job-pill">
      <Icon.Spinner size={14} style={{ color: "var(--pc-accent-text)" }} />
      <span>{label}</span>
      {job && (
        <span className="bar" style={{ width: 64 }} aria-hidden="true">
          <span style={{ width: `${job.overallPercent ?? 0}%` }} />
        </span>
      )}
    </button>
  );
}

export function RamChip() {
  const { t } = useTranslation();
  const res = useLive((s) => s.resources);
  const setModule = useLayout((s) => s.setModule);
  if (!res) return null;
  const { system, governor } = res;
  const used = system.totalMb - system.availableMb;
  const color =
    res.state === "normal"
      ? "var(--pc-ok)"
      : res.state === "pressure"
        ? "var(--pc-warn)"
        : "var(--pc-danger)";
  return (
    <button
      type="button"
      className="pill mono"
      style={{ fontSize: 12 }}
      onClick={() => setModule("jobs")}
      title={t("shell.ramTitle", {
        app: system.processMb,
        budget: governor.budgetMb,
        available: system.availableMb,
      })}
      data-testid="ram-chip"
    >
      <Icon.Ram size={14} style={{ color }} />
      <span>{t("shell.ram", { used: mbToGb(used), total: fmtDec(system.totalMb / 1024, 0) })}</span>
    </button>
  );
}

export function TopBar() {
  const { t } = useTranslation();
  return (
    <header
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: 16,
        padding: "8px 16px",
        background: "var(--pc-panel)",
        borderBottom: "1px solid var(--pc-border)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, fontWeight: 700, fontSize: 16 }}>
        <Icon.Logo size={22} style={{ color: "var(--pc-accent-text)" }} />
        <span>{t("app.name")}</span>
      </div>
      <label
        style={{
          flex: "1 1 280px",
          maxWidth: 520,
          display: "flex",
          alignItems: "center",
          gap: 8,
          background: "var(--pc-bg)",
          border: "1px solid var(--pc-border)",
          borderRadius: 6,
          padding: "0 10px",
          minHeight: 34,
          color: "var(--pc-text-secondary)",
        }}
      >
        <Icon.Search size={16} />
        <GlobalSearch />
      </label>
      <div
        style={{
          marginLeft: "auto",
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: 10,
        }}
      >
        <JobPill />
        <RamChip />
      </div>
    </header>
  );
}
