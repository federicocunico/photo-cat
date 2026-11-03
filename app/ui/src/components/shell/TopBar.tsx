import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { activeJobs, useLive } from "../../store/live";
import { useLayout } from "../../store/layout";
import { fmtInt } from "../../lib/format";
import { currentPhase, jobTitle } from "../../lib/jobs";
import { commands, isTauri } from "../../lib/ipc";
import { Dialog, Kbd, Menu } from "../ui";
import { usePalette } from "../../store/palette";
import { usePending } from "../../store/pending";
import { Spinner } from "../feedback/Spinner";

const MOD = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "") ? "⌘" : "Ctrl";

export function PendingIndicator() {
  const count = usePending((s) => s.count);
  const [visible, setVisible] = useState(false);
  const { t } = useTranslation();
  const active = count > 0;
  useEffect(() => {
    if (!active) {
      setVisible(false);
      return;
    }
    const timer = setTimeout(() => setVisible(true), 300);
    return () => clearTimeout(timer);
  }, [active]);
  return active && visible ? (
    <span title={t("feedback.pending", { count })} data-testid="pending-ipc">
      <Spinner label={t("feedback.pending", { count })} />
    </span>
  ) : null;
}

/**
 * Campo di ricerca della barra superiore: apre la palette dei comandi (Ctrl+K). Un
 * carattere digitato con il campo attivo diventa il testo iniziale della palette.
 */
function GlobalSearch() {
  const { t } = useTranslation();
  const openPalette = usePalette((s) => s.openPalette);
  return (
    <button
      type="button"
      className="topbar-search"
      aria-label={t("palette.field")}
      aria-keyshortcuts="Control+K"
      onClick={() => openPalette()}
      onKeyDown={(e) => {
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey && e.key !== " ") {
          e.preventDefault();
          openPalette(e.key);
        }
      }}
    >
      <Icon.Search size={16} aria-hidden="true" />
      <span className="topbar-search-text">{t("app.search")}</span>
      <Kbd keys={[MOD, "K"]} />
    </button>
  );
}

export function JobPill() {
  const { t } = useTranslation();
  const jobs = useLive((s) => s.jobs);
  const openModule = useLayout((s) => s.openModule);
  const job = activeJobs(jobs)[0];
  const percent = job ? Math.floor(job.overallPercent ?? 0) : 0;
  const phase = job ? currentPhase(job) : undefined;
  // In parole: che cosa sta facendo, mai un identificatore interno.
  const title = job ? jobTitle(t, job.kind, null) : "";
  const label = !job
    ? t("shell.noJobs")
    : phase && !phase.totalKnown
      ? t("shell.jobPillCounting", {
          title,
          found: t("jobs.indeterminate", { n: fmtInt(phase.done) }),
        })
      : t("shell.jobPill", { title, percent });
  return (
    <button
      type="button"
      className="pill"
      onClick={() => openModule(job ? { module: "jobs", target: job.id } : { module: "jobs" })}
      data-testid="job-pill"
    >
      {job ? <Spinner size={14} /> : <Icon.Jobs size={14} />}
      <span>{label}</span>
      {job && (
        <span className="bar" style={{ width: 64 }} aria-hidden="true">
          <span style={{ width: `${job.overallPercent ?? 0}%` }} />
        </span>
      )}
    </button>
  );
}

/** Menu Aiuto: le voci della guida si aggiungono con in-app-guidance (5.4). */
export function HelpMenu() {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const [about, setAbout] = useState(false);
  const info = useQuery({
    queryKey: ["app-info"],
    queryFn: () => commands.appInfo(),
    enabled: isTauri() && about,
  });
  return (
    <>
      <Menu
        align="end"
        trigger={
          <button type="button" className="btn btn-ghost icon-btn" aria-label={t("help.menu")}>
            <Icon.Help size={18} aria-hidden="true" />
          </button>
        }
        items={[
          {
            label: t("help.diagnostics"),
            icon: <Icon.Diagnostics size={14} />,
            onSelect: () => openModule({ module: "settings", section: "diagnostics" }),
          },
          { kind: "separator" },
          {
            label: t("help.about"),
            icon: <Icon.Info size={14} />,
            onSelect: () => setAbout(true),
          },
        ]}
      />
      <Dialog open={about} onOpenChange={setAbout} title={t("help.aboutTitle")}>
        <p>{t("help.tagline")}</p>
        {info.data && (
          <dl className="help-about">
            <dt>{t("help.version")}</dt>
            <dd>{info.data.version}</dd>
            <dt>{t("help.dataDir")}</dt>
            <dd className="mono">{info.data.dataDir}</dd>
          </dl>
        )}
      </Dialog>
    </>
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
      <GlobalSearch />
      <div
        style={{
          marginLeft: "auto",
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: 10,
        }}
      >
        <PendingIndicator />
        <JobPill />
        <HelpMenu />
      </div>
    </header>
  );
}
