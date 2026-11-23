import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { QueryView } from "../../components/feedback/QueryView";
import { Icon } from "../../components/icons";
import {
  Button,
  Card,
  Disclosure,
  EmptyState,
  FolderPicker,
  FormField,
  IconButton,
  InlineAlert,
  Menu,
  type MenuItem,
} from "../../components/ui";
import { ModulePage } from "../../components/shell/ModulePage";
import {
  commands,
  events,
  isTauri,
  type SourceSummary,
  type SourceWatchStatus,
} from "../../lib/ipc";
import { fmtDayMonth, fmtHourMinute, fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { activeJobs, useLive } from "../../store/live";
import { notify, notifyError } from "../../store/toasts";
import { FORMAT_GROUPS, splitPatterns } from "./formats";
import { RelinkDialog, RemoveMissingDialog, RemoveSourceDialog, UpdateSummaryView } from "./manage";
import { pickFolder, updateFolder, useImportUpdates } from "./sourceActions";
import "./import.css";

type DialogKind = { kind: "remove" | "relink" | "missing"; summary: SourceSummary } | null;

/** Apre Attività sull'attività indicata. */
const follow = (job: number) => useLayout.getState().openModule({ module: "jobs", target: job });

/** Avviso di avvio con il comando per seguire l'attività, senza lasciare la pagina. */
function notifyStarted(message: string, job: number, label: string) {
  notify({ tone: "ok", message, action: { label, run: () => follow(job) } });
}

/** Stato del monitoraggio per sorgente; si aggiorna a sources://watch-changed. */
function useWatchStatus() {
  const qc = useQueryClient();
  useEffect(() => {
    if (!isTauri()) return;
    let off: (() => void) | undefined;
    let gone = false;
    void events
      .onSourceWatchChanged(() => void qc.invalidateQueries({ queryKey: ["source-watch-status"] }))
      .then((fn) => (gone ? fn() : (off = fn)));
    return () => {
      gone = true;
      off?.();
    };
  }, [qc]);
  return useQuery({
    queryKey: ["source-watch-status"],
    queryFn: () => commands.sourceWatchStatus(),
  });
}

function WatchStatusLine({ status }: { status: SourceWatchStatus }) {
  const { t } = useTranslation();
  const at = status.lastAutoUpdateAt;
  // Solo l'ora se è di oggi, altrimenti anche il giorno ("ultimo il 5 gennaio alle 14:30").
  const today = at !== null && new Date(at).toDateString() === new Date().toDateString();
  const text =
    status.state === "active"
      ? at === null
        ? t("sources.watchStatus.active")
        : today
          ? t("sources.watchStatus.activeLast", { time: fmtHourMinute(at) })
          : t("sources.watchStatus.activeLastDay", {
              date: fmtDayMonth(at),
              time: fmtHourMinute(at),
            })
      : status.state === "unreachable"
        ? t("sources.watchStatus.unreachable")
        : t("sources.watchStatus.error");
  return (
    <span
      className={status.state === "active" ? "muted" : "import-warn"}
      // Il messaggio tecnico del sistema resta consultabile al passaggio del puntatore.
      title={status.state === "error" ? (status.message ?? undefined) : undefined}
      data-testid={`watch-status-${status.sourceId}`}
    >
      {text}
    </span>
  );
}

function FolderRow({
  summary,
  watchStatus,
  onDialog,
}: {
  summary: SourceSummary;
  watchStatus?: SourceWatchStatus;
  onDialog: (d: DialogKind) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const updates = useImportUpdates();
  const { source, items, missing, failedWarning } = summary;
  const newMissing = summary.newMissing ?? missing;
  const warned = newMissing > 0 || failedWarning > 0;
  const last = (updates.data ?? []).find((u) => u.sourceId === source.id);
  const update = async () => {
    const job = await commands.startImport(source.id, null);
    notifyStarted(t("import.updateStarted", { path: source.path }), job, t("import.openJob"));
  };
  const menuItems: MenuItem[] = [
    {
      label: t("sources.updateFolder"),
      icon: <Icon.Organize size={14} />,
      onSelect: async () => {
        try {
          const job = await updateFolder(source);
          if (job !== null) notifyStarted(t("sources.updateStarted"), job, t("import.openJob"));
        } catch (e) {
          notifyError(e);
        }
      },
    },
    ...(missing > 0
      ? ([
          { label: t("sources.relink"), onSelect: () => onDialog({ kind: "relink", summary }) },
          {
            label: t("sources.removeMissing"),
            onSelect: () => onDialog({ kind: "missing", summary }),
          },
        ] satisfies MenuItem[])
      : []),
    { kind: "separator" },
    {
      label: t("sources.remove"),
      danger: true,
      onSelect: () => onDialog({ kind: "remove", summary }),
    },
  ];
  return (
    <li className="import-folder" data-testid={`source-${source.id}`}>
      <div className="import-folder-head">
        <Icon.Import size={16} />
        <span className="mono import-path">{source.path}</span>
        <span className="muted">{t("import.itemsCount", { n: fmtInt(items) })}</span>
        {newMissing > 0 && (
          <span className="import-warn">
            <Icon.Alert size={14} />{" "}
            {t(newMissing < missing ? "sources.warn.newMissing" : "sources.warn.missing", {
              count: newMissing,
            })}
          </span>
        )}
        {failedWarning > 0 && (
          <span className="import-warn">
            <Icon.Alert size={14} /> {t("sources.warn.failed", { count: failedWarning })}
          </span>
        )}
        {warned && (
          <Button
            size="sm"
            aria-label={t("sources.warn.ignoreLabel", { name: source.path })}
            onClick={async () => {
              try {
                await commands.acknowledgeSourceWarnings(source.id);
                await qc.invalidateQueries({ queryKey: ["sources"] });
              } catch (err) {
                notifyError(err);
              }
            }}
          >
            {t("sources.warn.ignore")}
          </Button>
        )}
        <label className="import-check" title={t("import.watchHint")}>
          <input
            type="checkbox"
            checked={source.watch}
            onChange={async (e) => {
              try {
                await commands.updateSource({ ...source, watch: e.target.checked });
                await qc.invalidateQueries({ queryKey: ["sources"] });
                await qc.invalidateQueries({ queryKey: ["source-watch-status"] });
              } catch (err) {
                notifyError(err);
              }
            }}
          />
          {t("import.watch")}
        </label>
        <div className="import-folder-actions">
          <Button size="sm" icon={<Icon.Refresh size={14} />} onClick={update}>
            {t("import.rescan")}
          </Button>
          <Menu
            align="end"
            items={menuItems}
            trigger={
              <IconButton
                size="sm"
                label={t("sources.menu", { name: source.path })}
                icon={<Icon.More size={16} />}
              />
            }
          />
        </div>
      </div>
      {source.watch && watchStatus && <WatchStatusLine status={watchStatus} />}
      {last && (
        <div className="import-folder-summary">
          <span className="muted">{t("sources.summary.title")}:</span>
          <UpdateSummaryView summary={last} quiet={!warned} />
        </div>
      )}
    </li>
  );
}

function FoldersList({ sources }: { sources: SourceSummary[] }) {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState<DialogKind>(null);
  const watch = useWatchStatus();
  const close = () => setDialog(null);
  return (
    <Card title={t("import.sources")}>
      {sources.length === 0 ? (
        <EmptyState
          icon={<Icon.Import size={24} />}
          title={t("import.noFolders")}
          description={t("import.noFoldersHint")}
        />
      ) : (
        <ul className="import-folders">
          {sources.map((summary) => (
            <FolderRow
              key={summary.source.id}
              summary={summary}
              watchStatus={(watch.data ?? []).find((w) => w.sourceId === summary.source.id)}
              onDialog={setDialog}
            />
          ))}
        </ul>
      )}
      {dialog?.kind === "remove" && (
        <RemoveSourceDialog source={dialog.summary.source} onClose={close} />
      )}
      {dialog?.kind === "relink" && (
        <RelinkDialog
          source={dialog.summary.source}
          missing={dialog.summary.missing}
          onClose={close}
        />
      )}
      {dialog?.kind === "missing" && (
        <RemoveMissingDialog
          sourceId={dialog.summary.source.id}
          missing={dialog.summary.missing}
          onClose={close}
        />
      )}
    </Card>
  );
}

function ScanSummary() {
  const { t } = useTranslation();
  const jobs = useLive((s) => s.jobs);
  const job = activeJobs(jobs).find((j) => j.kind === "import");
  if (!job)
    return (
      <Card title={t("import.scanning")}>
        <p className="muted import-idle">{t("import.idle")}</p>
      </Card>
    );
  const ph = (n: string) => job.phases.find((p) => p.name === n);
  const scan = ph("scansione");
  const meta = ph("metadati");
  const write = ph("scrittura");
  return (
    <Card title={t("import.scanning")} aria-label={t("import.scanning")}>
      <dl className="import-scan">
        <dt className="muted">{t("import.found")}</dt>
        <dd>{fmtInt(scan?.done ?? 0)}</dd>
        <dt className="muted">{t("import.processed")}</dt>
        <dd>{fmtInt(write?.done ?? 0)}</dd>
        <dt className={job.failed > 0 ? "ui-text-danger" : "muted"}>{t("jobs.col.errors")}</dt>
        <dd className={job.failed > 0 ? "ui-text-danger" : undefined}>{fmtInt(job.failed)}</dd>
        <dt className="muted">{t("jobs.phases.metadati")}</dt>
        <dd>{meta?.percent != null ? `${Math.floor(meta.percent)}%` : "—"}</dd>
      </dl>
      <div>
        <Button variant="ghost" onClick={() => follow(job.id)}>
          {t("import.openJob")}
        </Button>
      </div>
    </Card>
  );
}

type StartError = { message: string; detail?: string };

export function ImportPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [path, setPath] = useState("");
  const [recursive, setRecursive] = useState(true);
  const [watch, setWatch] = useState(false);
  const [formats, setFormats] = useState(() =>
    Object.fromEntries(FORMAT_GROUPS.map((g) => [g.id, g.def])),
  );
  const [ignore, setIgnore] = useState("**/.thumbnails/**; **/$RECYCLE.BIN/**; *.tmp");
  const [error, setError] = useState<StartError | null>(null);
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: commands.listSources,
    enabled: isTauri(),
  });
  const noFormat = !Object.values(formats).some(Boolean);
  const disabledReason = !path.trim()
    ? t("import.needFolder")
    : noFormat
      ? t("import.needFormat")
      : undefined;

  const start = async () => {
    setError(null);
    const folder = path.trim();
    try {
      const ext = FORMAT_GROUPS.filter((g) => formats[g.id]).flatMap((g) => g.ext);
      const id = await commands.addSource({
        path: folder,
        recursive,
        includeFormats: ext,
        ignorePatterns: splitPatterns(ignore),
        watch,
      });
      const job = await commands.startImport(id, null);
      await qc.invalidateQueries({ queryKey: ["sources"] });
      setPath("");
      // Si resta su Importa: l'avviso porta ad Attività solo se l'utente lo chiede.
      notifyStarted(t("import.started", { path: folder }), job, t("import.openJob"));
    } catch (e) {
      const err = e as { detail?: string; code?: string };
      const detail = err.detail ?? String(e);
      setError(
        err.code === "NOT_FOUND" ? { message: t("import.notFound"), detail } : { message: detail },
      );
    }
  };

  return (
    <ModulePage
      title={t("modules.import")}
      subtitle={t("import.subtitle")}
      bodyClassName="import-body"
    >
      <div className="import-columns">
        <Card title={t("import.title")} className="import-form">
          <FolderPicker
            label={t("import.folder")}
            value={path}
            onChange={(p) => {
              setPath(p);
              setError(null);
            }}
            pick={() => pickFolder()}
          />
          {error && (
            <InlineAlert tone="danger" title={error.message}>
              {error.detail && (
                <Disclosure title={t("common.details")} defaultOpen={false} level={4}>
                  <p className="mono import-path">{error.detail}</p>
                </Disclosure>
              )}
            </InlineAlert>
          )}
          <div className="import-options">
            <label className="import-check">
              <input
                type="checkbox"
                checked={recursive}
                onChange={(e) => setRecursive(e.target.checked)}
              />
              {t("import.recursive")}
            </label>
            <label className="import-check" title={t("import.watchHint")}>
              <input type="checkbox" checked={watch} onChange={(e) => setWatch(e.target.checked)} />
              {t("import.watchNew")}
            </label>
          </div>
          <fieldset className="import-formats">
            <legend className="muted">{t("import.formats")}</legend>
            <div>
              {FORMAT_GROUPS.map((g) => (
                <label key={g.id} className="import-check">
                  <input
                    type="checkbox"
                    checked={formats[g.id]}
                    onChange={(e) => setFormats({ ...formats, [g.id]: e.target.checked })}
                  />
                  {g.label}
                </label>
              ))}
            </div>
          </fieldset>
          <Disclosure title={t("import.advanced")} defaultOpen={false}>
            <FormField label={t("import.ignore")} hint={t("import.ignoreHint")}>
              <input
                className="field mono"
                value={ignore}
                spellCheck={false}
                onChange={(e) => setIgnore(e.target.value)}
              />
            </FormField>
          </Disclosure>
          <div className="import-submit">
            <Button variant="primary" disabledReason={disabledReason} onClick={start}>
              {t("import.start")}
            </Button>
          </div>
        </Card>
        <div className="import-side">
          <ScanSummary />
        </div>
      </div>
      <QueryView query={sources} isEmpty={() => false}>
        {() => <FoldersList sources={sources.data ?? []} />}
      </QueryView>
    </ModulePage>
  );
}
