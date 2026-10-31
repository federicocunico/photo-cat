import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { commands, isTauri, type ErrorEntry, type LogLevel } from "../../lib/ipc";
import { toQuery, useErrorText } from "./errors";
import { fmtDateTime, fmtTime } from "../../lib/format";
import { useLayout, type DiagFilter } from "../../store/layout";
import { useLive } from "../../store/live";

const PHASES = [
  "scansione",
  "metadati",
  "miniature",
  "analisi_ai",
  "volti",
  "scrittura",
  "modelli",
];

function levelBadge(level: string) {
  const err = level === "ERROR";
  return {
    padding: "2px 8px",
    borderRadius: 4,
    fontSize: 12,
    background: err ? "var(--pc-danger-bg)" : "var(--pc-warn-bg)",
    color: err ? "var(--pc-danger)" : "var(--pc-warn)",
  } as const;
}

function Detail({ entry }: { entry: ErrorEntry }) {
  const { t, i18n } = useTranslation();
  const text = useErrorText();
  const hintKey = entry.code ? `hints.${entry.code.toLowerCase()}` : "";
  return (
    <aside
      aria-label={t("diag.detail")}
      style={{
        flex: "1 1 320px",
        minWidth: 0,
        background: "var(--pc-panel)",
        border: "1px solid var(--pc-border)",
        borderRadius: 8,
        padding: 18,
        display: "flex",
        flexDirection: "column",
        gap: 10,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={levelBadge(entry.level)}>{entry.level}</span>
        <strong className="mono">{entry.code ?? "—"}</strong>
      </div>
      <div data-testid="detail-message">{text(entry)}</div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
          gap: "6px 12px",
          fontSize: 13,
        }}
      >
        <span className="muted">{t("diag.file")}</span>
        <span className="mono" style={{ fontSize: 11, wordBreak: "break-all" }}>
          {entry.path || "—"}
        </span>
        <span className="muted">{t("diag.job")}</span>
        <span>{entry.jobId ?? "—"}</span>
        <span className="muted">{t("diag.phase")}</span>
        <span>
          {entry.phase ? t(`jobs.phases.${entry.phase}`, { defaultValue: entry.phase }) : "—"}
        </span>
        <span className="muted">{t("diag.time")}</span>
        <span>{fmtDateTime(entry.ts)}</span>
      </div>
      {hintKey && i18n.exists(hintKey) && (
        <div style={{ color: "var(--pc-warn)", fontSize: 13 }} data-testid="detail-hint">
          {t("diag.suggestion")}: {t(hintKey)}
        </div>
      )}
      <details>
        <summary style={{ cursor: "pointer" }}>{t("diag.technical")}</summary>
        <pre
          className="mono"
          style={{
            margin: "8px 0 0",
            padding: 10,
            background: "var(--pc-bg)",
            border: "1px solid var(--pc-border)",
            borderRadius: 6,
            fontSize: 11,
            color: "var(--pc-text-soft)",
            whiteSpace: "pre-wrap",
          }}
        >
          {[entry.target, entry.message, entry.detail].filter(Boolean).join("\n")}
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
    <section
      aria-label={t("diag.tail")}
      style={{
        background: "var(--pc-log-bg)",
        border: "1px solid var(--pc-border)",
        borderRadius: 8,
        padding: "12px 16px",
      }}
    >
      <div
        style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8, fontSize: 12 }}
        className="muted"
      >
        <span>{t("diag.tail")}</span>
        <label style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6 }}>
          <input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />
          {t("diag.follow")}
        </label>
      </div>
      <pre
        ref={ref}
        className="mono"
        style={{
          margin: 0,
          fontSize: 12,
          lineHeight: 1.6,
          color: "var(--pc-text-soft)",
          whiteSpace: "pre-wrap",
          maxHeight: 220,
          overflow: "auto",
        }}
      >
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

export function DiagnosticsPage() {
  const { t } = useTranslation();
  const text = useErrorText();
  const initial = useLayout((s) => s.diagFilter);
  const [filter, setFilter] = useState<DiagFilter>(initial);
  const [selected, setSelected] = useState<number | null>(null);
  const [level, setLevel] = useState<LogLevel | "">("");
  const resetErrors = useLive((s) => s.resetErrors);
  useEffect(() => setFilter(initial), [initial]);
  useEffect(() => resetErrors(), [resetErrors]);
  useEffect(() => {
    if (isTauri())
      commands
        .getConfig()
        .then((c) => setLevel(c.logging?.level ?? "info"))
        .catch(() => {});
  }, []);

  const query = useMemo(() => toQuery(filter), [filter]);
  const errors = useQuery({
    queryKey: ["errors", query],
    queryFn: () => commands.queryErrors(query),
    enabled: isTauri(),
    refetchInterval: 3000,
  });
  const rows = errors.data ?? [];
  const current = rows.find((r) => r.id === selected) ?? rows[0];
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const run = async (fn: () => Promise<string | void>) => {
    setBusy(true);
    setNotice("");
    try {
      setNotice((await fn()) ?? "");
    } catch (e) {
      setNotice((e as { detail?: string }).detail ?? String(e));
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
    <label style={{ display: "flex", alignItems: "center", gap: 6 }} className="muted">
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

  return (
    <div
      style={{
        flex: 1,
        overflow: "auto",
        padding: "24px 32px",
        display: "flex",
        flexDirection: "column",
        gap: 16,
      }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 10 }}>
        <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("diag.title")}</h1>
        <div style={{ marginLeft: "auto", display: "flex", flexWrap: "wrap", gap: 8 }}>
          {select(
            t("diag.logLevel"),
            level,
            ["error", "warn", "info", "debug", "trace"].map((l) => [l, l]),
            (v) => {
              setLevel(v as LogLevel);
              void commands.setLogLevel(v as LogLevel);
            },
          )}
          <button
            type="button"
            className="btn"
            disabled={busy || rows.length === 0}
            onClick={() =>
              void run(async () => {
                const jobs = await commands.retryFailed(filter.jobId ?? null, filter.phase ?? null);
                return t("diag.retried", { n: jobs.length });
              })
            }
          >
            {t("diag.retryFailed")}
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => void run(async () => void (await commands.openLogs()))}
          >
            {t("diag.openLogs")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={() =>
              void run(async () => {
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
          </button>
        </div>
      </div>
      {notice && (
        <p role="status" className="muted" style={{ margin: 0 }}>
          {notice}
        </p>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 10, alignItems: "center" }}>
        {select(
          t("diag.level"),
          filter.level ?? "",
          [
            ["", t("diag.any")],
            ["ERROR", "ERROR"],
            ["WARN", "WARN"],
          ],
          (v) => setFilter({ ...filter, level: v || undefined }),
        )}
        {select(
          t("diag.phase"),
          filter.phase ?? "",
          [
            ["", t("diag.any")],
            ...PHASES.map(
              (p) => [p, t(`jobs.phases.${p}`, { defaultValue: p })] as [string, string],
            ),
          ],
          (v) => setFilter({ ...filter, phase: v || undefined }),
        )}
        <label style={{ display: "flex", alignItems: "center", gap: 6 }} className="muted">
          {t("diag.job")}
          <input
            className="field"
            style={{ width: 90 }}
            inputMode="numeric"
            aria-label={t("diag.job")}
            value={filter.jobId ?? ""}
            onChange={(e) => {
              const n = parseInt(e.target.value, 10);
              setFilter({ ...filter, jobId: Number.isFinite(n) ? n : undefined });
            }}
          />
        </label>
        <label
          className="field"
          style={{ flex: "1 1 220px", display: "flex", alignItems: "center", gap: 8 }}
        >
          <Icon.Search size={14} />
          <input
            aria-label={t("diag.text")}
            placeholder={t("diag.text")}
            value={filter.text ?? ""}
            onChange={(e) => setFilter({ ...filter, text: e.target.value })}
            style={{ flex: 1, background: "transparent", border: 0, outline: "none" }}
          />
        </label>
        <button type="button" className="btn btn-ghost" onClick={() => setFilter({})}>
          {t("diag.clearFilters")}
        </button>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 16, alignItems: "flex-start" }}>
        <section
          style={{
            flex: "999 1 560px",
            minWidth: 0,
            background: "var(--pc-panel)",
            border: "1px solid var(--pc-border)",
            borderRadius: 8,
            overflow: "auto",
            maxHeight: 520,
          }}
        >
          <table className="table" style={{ minWidth: 760 }}>
            <thead>
              <tr>
                <th>{t("diag.time")}</th>
                <th>{t("diag.level")}</th>
                <th>{t("diag.phase")}</th>
                <th>{t("diag.file")}</th>
                <th>{t("diag.message")}</th>
                <th>{t("diag.code")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.length === 0 && (
                <tr>
                  <td colSpan={6} className="muted" style={{ padding: 16 }}>
                    {t("diag.empty")}
                  </td>
                </tr>
              )}
              {rows.map((r) => (
                <tr
                  key={r.id}
                  aria-selected={current?.id === r.id}
                  onClick={() => setSelected(r.id)}
                  style={{ cursor: "pointer" }}
                  data-testid={`error-row-${r.id}`}
                >
                  <td className="mono" style={{ fontSize: 12 }}>
                    {fmtTime(r.ts)}
                  </td>
                  <td>
                    <span style={levelBadge(r.level)}>{r.level}</span>
                  </td>
                  <td>{r.phase ? t(`jobs.phases.${r.phase}`, { defaultValue: r.phase }) : "—"}</td>
                  <td className="mono" style={{ fontSize: 12, wordBreak: "break-all" }}>
                    {r.path ? r.path.split(/[\\/]/).pop() : "—"}
                  </td>
                  <td>{text(r)}</td>
                  <td className="mono muted" style={{ fontSize: 12 }}>
                    {r.code ?? "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
        {current && <Detail entry={current} />}
      </div>
      <LogTail />
    </div>
  );
}
