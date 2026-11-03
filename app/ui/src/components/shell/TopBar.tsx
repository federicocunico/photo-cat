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
import { useShortcutSheet } from "../../store/shortcutSheet";
import { useGuide } from "../../store/guide";
import { notify } from "../../store/toasts";
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
        <span className="bar job-pill-bar" aria-hidden="true">
          <span style={{ width: `${job.overallPercent ?? 0}%` }} />
        </span>
      )}
    </button>
  );
}

/** Menu Aiuto: scorciatoie, introduzione, primi passi, suggerimenti, guida, problemi, informazioni. */
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
            label: t("commands.shortcuts"),
            icon: <Icon.Keyboard size={14} />,
            shortcut: "?",
            onSelect: () => useShortcutSheet.getState().setOpen(true),
          },
          {
            label: t("help.intro"),
            icon: <Icon.Refresh size={14} />,
            onSelect: () => useGuide.getState().setIntro(true),
          },
          {
            label: t("help.firstSteps"),
            icon: <Icon.Check size={14} />,
            onSelect: () => {
              useGuide.getState().show();
              openModule({ module: "library" });
            },
          },
          {
            label: t("help.tips"),
            icon: <Icon.Info size={14} />,
            onSelect: () => {
              useGuide.getState().resetTips();
              notify({ tone: "ok", message: t("guide.tipsReset") });
            },
          },
          {
            label: t("help.guide"),
            icon: <Icon.Help size={14} />,
            onSelect: () => useGuide.getState().openGuide("library"),
          },
          { kind: "separator" },
          {
            label: t("help.report"),
            icon: <Icon.Alert size={14} />,
            onSelect: () => {
              useGuide.getState().setReport(true);
              openModule({ module: "settings", section: "diagnostics" });
            },
          },
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
    <header className="topbar">
      <div className="topbar-brand">
        <Icon.Logo size={22} className="topbar-brand-icon" />
        <span>{t("app.name")}</span>
      </div>
      <GlobalSearch />
      <div className="topbar-end">
        <PendingIndicator />
        <JobPill />
        <HelpMenu />
      </div>
    </header>
  );
}
