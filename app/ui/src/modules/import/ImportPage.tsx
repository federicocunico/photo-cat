import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type SourceSummary } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { activeJobs, useLive } from "../../store/live";
import { FORMAT_GROUPS, splitPatterns } from "./formats";

const card = {
  background: "var(--pc-panel)",
  border: "1px solid var(--pc-border)",
  borderRadius: 8,
  padding: 24,
} as const;

async function pickFolder(): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({ directory: true, multiple: false });
  return typeof res === "string" ? res : null;
}

function SourcesList({ sources }: { sources: SourceSummary[] }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const refresh = () => qc.invalidateQueries({ queryKey: ["sources"] });
  if (sources.length === 0) return null;
  return (
    <section style={{ ...card, display: "flex", flexDirection: "column", gap: 10 }}>
      <h2 style={{ margin: 0, fontSize: 16 }}>{t("import.sources")}</h2>
      {sources.map(({ source, items, missing }) => (
        <div
          key={source.id}
          data-testid={`source-${source.id}`}
          style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}
        >
          <span className="mono" style={{ fontSize: 13, wordBreak: "break-all" }}>
            {source.path}
          </span>
          <span className="muted">{t("import.itemsCount", { n: fmtInt(items) })}</span>
          {missing > 0 && (
            <span style={{ color: "var(--pc-warn)" }}>
              {t("import.missing", { n: fmtInt(missing) })}
            </span>
          )}
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <input
              type="checkbox"
              checked={source.watch}
              onChange={async (e) => {
                await commands.updateSource({ ...source, watch: e.target.checked });
                await refresh();
              }}
            />
            {t("import.watch")}
          </label>
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            <button type="button" className="btn" onClick={() => commands.startImport(source.id)}>
              {t("import.rescan")}
            </button>
            <button
              type="button"
              className="btn btn-danger"
              onClick={async () => {
                if (
                  !window.confirm(
                    t("import.confirmRemove", { path: source.path, n: fmtInt(items) }),
                  )
                )
                  return;
                await commands.removeSource(source.id);
                await refresh();
              }}
            >
              {t("import.remove")}
            </button>
          </div>
        </div>
      ))}
    </section>
  );
}

function ScanSummary() {
  const { t } = useTranslation();
  const jobs = useLive((s) => s.jobs);
  const setModule = useLayout((s) => s.setModule);
  const job = activeJobs(jobs).find((j) => j.kind === "import");
  if (!job) return null;
  const ph = (n: string) => job.phases.find((p) => p.name === n);
  const scan = ph("scansione");
  const meta = ph("metadati");
  const write = ph("scrittura");
  return (
    <section
      aria-label={t("import.scanning")}
      style={{ ...card, padding: 20, display: "flex", flexDirection: "column", gap: 14 }}
    >
      <h2 style={{ margin: 0, fontSize: 16 }}>{t("import.scanning")}</h2>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
          gap: "8px 12px",
        }}
      >
        <span className="muted">{t("import.found")}</span>
        <span className="mono">{fmtInt(scan?.done ?? 0)}</span>
        <span className="muted">{t("import.processed")}</span>
        <span className="mono">{fmtInt(write?.done ?? 0)}</span>
        <span style={{ color: "var(--pc-danger)" }}>{t("jobs.col.errors")}</span>
        <span className="mono" style={{ color: "var(--pc-danger)" }}>
          {fmtInt(job.failed)}
        </span>
        <span className="muted">{t("jobs.phases.metadati")}</span>
        <span className="mono">{meta?.percent != null ? `${Math.floor(meta.percent)}%` : "—"}</span>
      </div>
      <button
        type="button"
        className="btn btn-ghost"
        style={{ alignSelf: "flex-start" }}
        onClick={() => setModule("jobs")}
      >
        {t("import.openJob")}
      </button>
    </section>
  );
}

export function ImportPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const setModule = useLayout((s) => s.setModule);
  const [path, setPath] = useState("");
  const [recursive, setRecursive] = useState(true);
  const [watch, setWatch] = useState(false);
  const [formats, setFormats] = useState(() =>
    Object.fromEntries(FORMAT_GROUPS.map((g) => [g.id, g.def])),
  );
  const [ignore, setIgnore] = useState("**/.thumbnails/**; **/$RECYCLE.BIN/**; *.tmp");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: commands.listSources,
    enabled: isTauri(),
  });

  const start = async () => {
    setError(null);
    setBusy(true);
    try {
      const ext = FORMAT_GROUPS.filter((g) => formats[g.id]).flatMap((g) => g.ext);
      const id = await commands.addSource({
        path: path.trim(),
        recursive,
        includeFormats: ext,
        ignorePatterns: splitPatterns(ignore),
        watch,
      });
      await commands.startImport(id);
      await qc.invalidateQueries({ queryKey: ["sources"] });
      setPath("");
      setModule("jobs");
    } catch (e) {
      const err = e as { detail?: string; messageKey?: string };
      setError(err.detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      style={{
        flex: 1,
        overflow: "auto",
        padding: 32,
        display: "flex",
        flexDirection: "column",
        gap: 24,
      }}
    >
      <div>
        <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("import.title")}</h1>
        <p className="muted" style={{ margin: "6px 0 0" }}>
          {t("import.subtitle")}
        </p>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 24, alignItems: "flex-start" }}>
        <section
          style={{
            ...card,
            flex: "2 1 520px",
            minWidth: 0,
            display: "flex",
            flexDirection: "column",
            gap: 20,
          }}
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <label htmlFor="src" style={{ fontWeight: 600 }}>
              {t("import.folder")}
            </label>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <input
                id="src"
                className="field mono"
                style={{ flex: "1 1 300px", minHeight: 40 }}
                value={path}
                placeholder="D:\Foto"
                onChange={(e) => setPath(e.target.value)}
              />
              <button
                type="button"
                className="btn"
                style={{ minHeight: 40 }}
                onClick={async () => {
                  const p = await pickFolder();
                  if (p) setPath(p);
                }}
              >
                {t("import.browse")}
              </button>
            </div>
          </div>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
              gap: "12px 24px",
            }}
          >
            <label style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <input
                type="checkbox"
                checked={recursive}
                onChange={(e) => setRecursive(e.target.checked)}
              />
              {t("import.recursive")}
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <input type="checkbox" checked={watch} onChange={(e) => setWatch(e.target.checked)} />
              {t("import.watchNew")}
            </label>
          </div>
          <fieldset
            style={{
              border: "1px solid var(--pc-border)",
              borderRadius: 6,
              padding: "12px 16px",
              margin: 0,
            }}
          >
            <legend className="muted" style={{ padding: "0 6px" }}>
              {t("import.formats")}
            </legend>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 16 }}>
              {FORMAT_GROUPS.map((g) => (
                <label key={g.id} style={{ display: "flex", gap: 6, alignItems: "center" }}>
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
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <label htmlFor="ign" style={{ fontWeight: 600 }}>
              {t("import.ignore")}
            </label>
            <input
              id="ign"
              className="field mono"
              style={{ minHeight: 40 }}
              value={ignore}
              onChange={(e) => setIgnore(e.target.value)}
            />
            <span className="muted" style={{ fontSize: 12 }}>
              {t("import.ignoreHint")}
            </span>
          </div>
          {error && (
            <div role="alert" style={{ color: "var(--pc-danger)" }}>
              {error}
            </div>
          )}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 12 }}>
            <button
              type="button"
              className="btn btn-primary"
              style={{ minHeight: 44, padding: "0 20px" }}
              disabled={busy || !path.trim() || !Object.values(formats).some(Boolean)}
              onClick={start}
            >
              {t("import.start")}
            </button>
          </div>
        </section>
        <div
          style={{
            flex: "1 1 300px",
            minWidth: 0,
            display: "flex",
            flexDirection: "column",
            gap: 24,
          }}
        >
          <ScanSummary />
        </div>
      </div>
      <SourcesList sources={sources.data ?? []} />
    </div>
  );
}
