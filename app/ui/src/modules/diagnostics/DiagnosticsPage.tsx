// Impostazioni › Diagnostica: segnalazione di un problema (pacchetto diagnostico), livello di
// log, errori registrati (riepilogo per codice e tabella da tastiera con il dettaglio) e log.
import "./diagnostics.css";
import { useGuide } from "../../store/guide";
import { QueryView } from "../../components/feedback/QueryView";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useContext, useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { Badge, Button, DataTable, FormField, InlineAlert, type Column } from "../../components/ui";
import { commands, events, isTauri, type ErrorEntry, type LogLevel } from "../../lib/ipc";
import { ENTRY_LEVELS, LOG_LEVELS, PHASES, toQuery, useEntryLabels, useErrorText } from "./errors";
import { ErrorGroups } from "./ErrorGroups";
import { fmtDateTime, fmtTime } from "../../lib/format";
import { useLayout, type DiagFilter } from "../../store/layout";
import { useLive } from "../../store/live";
import { SettingsCtx } from "../settings/settingsContext";

const levelTone = (level: string) => (level === "ERROR" ? "danger" : "warn");

function Detail({ entry }: { entry: ErrorEntry }) {
  const { t, i18n } = useTranslation();
  const text = useErrorText();
  const labels = useEntryLabels();
  const hintKey = entry.code ? `hints.${entry.code.toLowerCase()}` : "";
  return (
    <aside aria-label={t("diag.detail")} className="diag-detail">
      <div className="diag-detail-head">
        <Badge tone={levelTone(entry.level)}>{labels.level(entry.level)}</Badge>
        <strong data-testid="detail-message">{text(entry)}</strong>
      </div>
      <dl className="diag-detail-grid">
        <dt className="muted">{t("diag.file")}</dt>
        <dd className="mono diag-path">{entry.path || "—"}</dd>
        <dt className="muted">{t("diag.job")}</dt>
        <dd>{entry.jobId ?? "—"}</dd>
        <dt className="muted">{t("diag.phase")}</dt>
        <dd>{labels.phase(entry.phase)}</dd>
        <dt className="muted">{t("diag.time")}</dt>
        <dd>{fmtDateTime(entry.ts)}</dd>
      </dl>
      {hintKey && i18n.exists(hintKey) && (
        <p className="diag-hint" data-testid="detail-hint">
          {t("diag.suggestion")}: {t(hintKey)}
        </p>
      )}
      <details>
        <summary>{t("diag.technical")}</summary>
        <pre className="mono diag-technical">
          {[
            entry.code && `${t("diag.code")}: ${entry.code}`,
            entry.target,
            entry.message,
            entry.detail,
          ]
            .filter(Boolean)
            .join("\n")}
        </pre>
      </details>
    </aside>
  );
}

function LogTail() {
  const { t } = useTranslation();
  const tail = useLive((s) => s.tail);
  const [follow, setFollow] = useState(true);
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => {
    if (follow && ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [tail, follow]);
  return (
    <section aria-label={t("diag.tail")} className="diag-tail">
      <div className="diag-tail-head muted">
        <span>{t("diag.tail")}</span>
        <label>
          <input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />
          {t("diag.follow")}
        </label>
      </div>
      <pre ref={ref} className="mono">
        {tail
          .map((l) => {
            const time = fmtTime(l.ts);
            const fields = l.fields.map(([k, v]) => `${k}=${v}`).join(" ");
            return `${time} ${l.level.padEnd(5)} ${l.target.split("::").pop()} ${l.message} ${fields}`;
          })
          .join("\n")}
      </pre>
    </section>
  );
}

type Notice = { tone: "ok" | "danger"; text: string };

export function DiagnosticsPage() {
  const { t } = useTranslation();
  const text = useErrorText();
  const labels = useEntryLabels();
  const initial = useLayout((s) => s.diagFilter);
  const [filter, setFilter] = useState<DiagFilter>(initial);
  const [selected, setSelected] = useState<number | null>(null);
  const [level, setLevel] = useState<LogLevel | "">("");
  const resetErrors = useLive((s) => s.resetErrors);
  useEffect(() => setFilter(initial), [initial]);
  useEffect(() => resetErrors(), [resetErrors]);
  // Dalla ricerca delle Impostazioni ("log"): il campo "Livello di log" è evidenziato.
  const highlight = useContext(SettingsCtx)?.highlight;
  const report = useGuide((g) => g.report);
  const exportRef = useRef<HTMLButtonElement>(null);
  const detailRef = useRef<HTMLDivElement>(null);
  const logLevelId = useId();
  useEffect(() => {
    if (!report) return;
    exportRef.current?.focus();
    exportRef.current?.scrollIntoView?.({ block: "center" });
    const timer = setTimeout(() => useGuide.getState().setReport(false), 4000);
    return () => clearTimeout(timer);
  }, [report]);
  useEffect(() => {
    if (isTauri())
      commands
        .getConfig()
        .then((c) => setLevel(c.logging?.level ?? "info"))
        .catch(() => {});
  }, []);

  const query = useMemo(() => toQuery(filter), [filter]);
  const qc = useQueryClient();
  // Aggiornamento sui lotti di errori del backend, non a intervalli.
  useEffect(() => {
    const off = events.onErrorBatch(() => {
      void qc.invalidateQueries({ queryKey: ["errors"] });
      void qc.invalidateQueries({ queryKey: ["error-groups"] });
    });
    return () => {
      void off.then((f) => f());
    };
  }, [qc]);
  const errors = useQuery({
    queryKey: ["errors", query],
    queryFn: () => commands.queryErrors(query),
    enabled: isTauri(),
  });
  const rows = errors.data ?? [];
  const current = rows.find((r) => r.id === selected) ?? rows[0];
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const run = async (fn: () => Promise<string | void>) => {
    setBusy(true);
    setNotice(null);
    try {
      const done = await fn();
      if (done) setNotice({ tone: "ok", text: done });
    } catch (e) {
      setNotice({ tone: "danger", text: (e as { detail?: string }).detail ?? String(e) });
    } finally {
      setBusy(false);
    }
  };

  const select = (
    label: string,
    value: string,
    options: Array<[string, string]>,
    onChange: (v: string) => void,
  ) => (
    <label className="diag-filter muted">
      {label}
      <select className="field" value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map(([v, l]) => (
          <option key={v} value={v}>
            {l}
          </option>
        ))}
      </select>
    </label>
  );

  const columns: Column<ErrorEntry>[] = [
    {
      key: "time",
      header: t("diag.time"),
      width: "90px",
      cell: (r) => <span className="mono diag-small">{fmtTime(r.ts)}</span>,
    },
    {
      key: "level",
      header: t("diag.level"),
      width: "100px",
      cell: (r) => <Badge tone={levelTone(r.level)}>{labels.level(r.level)}</Badge>,
    },
    { key: "phase", header: t("diag.phase"), width: "130px", cell: (r) => labels.phase(r.phase) },
    {
      key: "file",
      header: t("diag.file"),
      cell: (r) => (
        <span className="mono diag-small diag-path">
          {r.path ? r.path.split(/[\\/]/).pop() : "—"}
        </span>
      ),
    },
    { key: "message", header: t("diag.message"), cell: (r) => text(r) },
  ];

  return (
    <div className="diag-page">
      <header className="diag-head">
        <h2>{t("diag.title")}</h2>
        <p className="muted">{t("diag.description")}</p>
      </header>

      <section className="ui-card diag-report" aria-labelledby="diag-report-title">
        <div className="diag-report-text">
          <h3 id="diag-report-title">
            <Icon.Help size={18} /> {t("diag.report.title")}
          </h3>
          <p className="muted">{t("diag.report.text")}</p>
        </div>
        <div className="diag-actions">
          <Button
            ref={exportRef}
            variant="primary"
            icon={<Icon.Export size={16} />}
            className={report ? "ui-highlight" : undefined}
            disabled={busy}
            onClick={async () =>
              await run(async () => {
                const { save } = await import("@tauri-apps/plugin-dialog");
                const path = await save({
                  defaultPath: `photo-cat-diagnostica-${new Date().toISOString().slice(0, 10)}.zip`,
                  filters: [{ name: "ZIP", extensions: ["zip"] }],
                });
                if (typeof path !== "string") return;
                await commands.exportDiagnostics(path);
                return t("diag.exported", { path });
              })
            }
          >
            {t("diag.exportBundle")}
          </Button>
          <Button
            icon={<Icon.OpenExternal size={16} />}
            disabled={busy}
            onClick={async () => await run(async () => void (await commands.openLogs()))}
          >
            {t("diag.openLogs")}
          </Button>
        </div>
      </section>

      {notice && (
        <InlineAlert
          tone={notice.tone}
          title={notice.tone === "danger" ? t("diag.failed") : undefined}
          onDismiss={() => setNotice(null)}
        >
          {notice.text}
        </InlineAlert>
      )}

      <div
        data-setting="logLevel"
        className={highlight === "logLevel" ? "diag-loglevel ui-highlight" : "diag-loglevel"}
      >
        <FormField label={t("diag.logLevel")} hint={t("diag.logLevelHint")}>
          <select
            id={logLevelId}
            className="field"
            value={level}
            onChange={(e) => {
              const v = e.target.value as LogLevel;
              setLevel(v);
              void commands.setLogLevel(v);
            }}
          >
            {level === "" && <option value="" />}
            {LOG_LEVELS.map((l) => (
              <option key={l} value={l}>
                {t(`diag.logLevels.${l}`)}
              </option>
            ))}
          </select>
        </FormField>
      </div>

      <section className="ui-card diag-errors" aria-labelledby="diag-errors-title">
        <div className="diag-errors-head">
          <h3 id="diag-errors-title">{t("diag.errorsTitle")}</h3>
          <Button
            icon={<Icon.Refresh size={16} />}
            disabled={busy}
            disabledReason={rows.length === 0 ? t("diag.nothingToRetry") : undefined}
            onClick={async () =>
              await run(async () => {
                const jobs = await commands.retryFailed(filter.jobId ?? null, filter.phase ?? null);
                return t("diag.retried", { n: jobs.length });
              })
            }
          >
            {t("diag.retryFailed")}
          </Button>
        </div>
        <div className="diag-filters" role="group" aria-label={t("diag.filters")}>
          {select(
            t("diag.level"),
            filter.level ?? "",
            [
              ["", t("diag.any")],
              ...ENTRY_LEVELS.map(
                (l) => [l, t(`diag.levelFilter.${l.toLowerCase()}`)] as [string, string],
              ),
            ],
            (v) => setFilter({ ...filter, level: v || undefined }),
          )}
          {select(
            t("diag.phase"),
            filter.phase ?? "",
            [["", t("diag.any")], ...PHASES.map((p) => [p, labels.phase(p)] as [string, string])],
            (v) => setFilter({ ...filter, phase: v || undefined }),
          )}
          <label className="diag-filter muted">
            {t("diag.job")}
            <input
              className="field diag-job"
              inputMode="numeric"
              value={filter.jobId ?? ""}
              onChange={(e) => {
                const n = parseInt(e.target.value, 10);
                setFilter({ ...filter, jobId: Number.isFinite(n) ? n : undefined });
              }}
            />
          </label>
          <label className="field diag-search">
            <Icon.Search size={14} />
            <input
              aria-label={t("diag.text")}
              placeholder={t("diag.text")}
              value={filter.text ?? ""}
              onChange={(e) => setFilter({ ...filter, text: e.target.value })}
            />
          </label>
          <Button variant="ghost" onClick={() => setFilter({})}>
            {t("diag.clearFilters")}
          </Button>
        </div>
        <ErrorGroups query={query} />
        <div className="diag-split">
          <div className="diag-table">
            <QueryView query={errors} empty={<p className="muted diag-empty">{t("diag.empty")}</p>}>
              {() => (
                <DataTable
                  label={t("diag.errorsTitle")}
                  columns={columns}
                  rows={rows}
                  rowId={(r) => r.id}
                  selected={current?.id ?? null}
                  onSelect={(r) => setSelected(r.id)}
                  onActivate={(r) => {
                    setSelected(r.id);
                    detailRef.current?.scrollIntoView?.({ block: "nearest" });
                  }}
                />
              )}
            </QueryView>
          </div>
          {current && (
            <div ref={detailRef} className="diag-detail-wrap">
              <Detail entry={current} />
            </div>
          )}
        </div>
      </section>
      <LogTail />
    </div>
  );
}
