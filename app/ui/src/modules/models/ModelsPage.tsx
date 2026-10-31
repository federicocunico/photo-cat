import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  isTauri,
  type ExecutionProvider,
  type ModelState,
  type ModelStatus,
  type ResourceProfile,
} from "../../lib/ipc";
import { fmtBytes, fmtDateTime, fmtDec, fmtInt, mbToGb } from "../../lib/format";

const card = {
  background: "var(--pc-panel)",
  border: "1px solid var(--pc-border)",
  borderRadius: 8,
  padding: 20,
} as const;

const badge = (bg: string, fg: string) =>
  ({
    padding: "2px 8px",
    borderRadius: 4,
    background: bg,
    color: fg,
    fontSize: 12,
    alignSelf: "flex-start",
  }) as const;

function stateBadge(s: ModelState) {
  switch (s.state) {
    case "verified":
      return badge("var(--pc-ok-bg)", "var(--pc-ok)");
    case "downloading":
    case "update_available":
      return badge("var(--pc-accent-soft-bg)", "var(--pc-accent-soft-text)");
    case "missing":
      return badge("var(--pc-warn-bg)", "var(--pc-warn)");
    default:
      return badge("var(--pc-danger-bg)", "var(--pc-danger)");
  }
}

async function pickModelFile(filterName: string): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({
    multiple: false,
    filters: [{ name: filterName, extensions: ["onnx", "json", "tsv"] }],
  });
  return typeof res === "string" ? res : null;
}

function useModels() {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const list = useQuery({ queryKey: ["models"], queryFn: commands.listModels, enabled: isTauri() });
  useEffect(() => {
    if (list.data) setModels(list.data);
  }, [list.data]);
  useEffect(() => {
    const un = events.onModels(setModels);
    return () => {
      un.then((f) => f());
    };
  }, []);
  return models;
}

function ModelRow({ m }: { m: ModelStatus }) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const s = m.state;
  const pct =
    s.state === "downloading"
      ? Math.floor((s.done_bytes / Math.max(1, s.total_bytes)) * 100)
      : null;
  const label =
    s.state === "downloading"
      ? t("models.states.downloading", { pct })
      : t(`models.states.${s.state}`);
  const install = async () => {
    setError(null);
    try {
      const p = await pickModelFile(t("models.fileFilter"));
      if (!p) return;
      await commands.installModelFile(m.entry.id, p);
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  return (
    <tr style={{ borderTop: "1px solid var(--pc-border)" }} data-testid={`model-${m.entry.id}`}>
      <td style={{ padding: "12px 8px 12px 0", fontWeight: 600 }}>{m.entry.name}</td>
      <td style={{ padding: "12px 8px", color: "var(--pc-text-soft)" }}>
        {t(`models.roles.${m.entry.role}`)}
      </td>
      <td className="mono" style={{ padding: "12px 8px", fontSize: 12 }}>
        {m.entry.version}
      </td>
      <td className="mono" style={{ padding: "12px 8px", fontSize: 12 }}>
        {fmtBytes(m.entry.sizeBytes)}
      </td>
      <td
        className="mono muted"
        style={{ padding: "12px 8px", fontSize: 12 }}
        title={m.entry.sha256}
      >
        {m.entry.sha256.slice(0, 4)}…{m.entry.sha256.slice(-4)}
      </td>
      <td style={{ padding: "12px 8px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <span style={stateBadge(s)}>{label}</span>
          {pct !== null && (
            <>
              <div className="bar" style={{ width: 120 }}>
                <span style={{ width: `${pct}%` }} />
              </div>
              {s.state === "downloading" && (
                <span className="mono muted" style={{ fontSize: 12 }}>
                  {fmtBytes(s.done_bytes)} / {fmtBytes(s.total_bytes)} ·{" "}
                  {fmtBytes(s.bytes_per_second ?? 0)}/s
                </span>
              )}
            </>
          )}
          {(s.state === "error" || s.state === "self_test_failed") && (
            <span className="muted" style={{ fontSize: 12 }}>
              {s.message}
            </span>
          )}
          {error && (
            <span role="alert" style={{ color: "var(--pc-danger)", fontSize: 12 }}>
              {error}
            </span>
          )}
        </div>
      </td>
      <td style={{ padding: "12px 8px", fontSize: 12 }} className="muted">
        {m.selfTest
          ? `${m.selfTest.passed ? t("models.passed") : t("models.failed")} · ${fmtDec(m.selfTest.avgMs ?? 0, 1)} ms`
          : "—"}
      </td>
      <td style={{ padding: "12px 8px" }}>
        <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
          {s.state !== "verified" && s.state !== "downloading" && (
            <button
              type="button"
              className="btn"
              onClick={() => run(() => commands.downloadModel(m.entry.id))}
            >
              {t("models.download")}
            </button>
          )}
          <button
            type="button"
            className="btn btn-ghost"
            disabled={s.state !== "verified" && s.state !== "self_test_failed"}
            onClick={() => run(() => commands.selfTestModels([m.entry.id]))}
          >
            {t("models.test")}
          </button>
          <button
            type="button"
            className="btn btn-ghost"
            disabled={s.state === "downloading"}
            onClick={install}
          >
            {t("models.installFile")}
          </button>
        </div>
      </td>
    </tr>
  );
}

export function ModelsPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const models = useModels();
  const [error, setError] = useState<string | null>(null);
  const info = useQuery({
    queryKey: ["system-info"],
    queryFn: commands.systemInfo,
    enabled: isTauri(),
  });
  const ready = models.filter((m) => m.state.state === "verified").length;
  const total = models.reduce((a, m) => a + m.entry.sizeBytes, 0);
  const tested = models.filter((m) => m.selfTest);
  const sys = info.data?.system;

  const setProvider = async (p: ExecutionProvider) => {
    await commands.setExecutionProvider(p);
    await qc.invalidateQueries({ queryKey: ["system-info"] });
  };
  const setProfile = async (p: ResourceProfile) => {
    await commands.setResourceProfile(p);
    await qc.invalidateQueries({ queryKey: ["system-info"] });
  };
  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };

  return (
    <div
      style={{
        flex: 1,
        overflow: "auto",
        padding: "24px 32px",
        display: "flex",
        flexDirection: "column",
        gap: 20,
      }}
    >
      <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("models.title")}</h1>
      {error && (
        <p role="alert" style={{ color: "var(--pc-danger)", margin: 0 }}>
          {error}
        </p>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 20 }}>
        <section
          style={{ ...card, flex: "1 1 360px", display: "flex", flexDirection: "column", gap: 12 }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <h2 style={{ margin: 0, fontSize: 16 }}>{t("models.system")}</h2>
            {info.data && (
              <span
                style={{
                  marginLeft: "auto",
                  ...badge(
                    info.data.onnxRuntime ? "var(--pc-ok-bg)" : "var(--pc-danger-bg)",
                    info.data.onnxRuntime ? "var(--pc-ok)" : "var(--pc-danger)",
                  ),
                }}
              >
                {info.data.onnxRuntime ? t("models.runtimeOk") : t("models.runtimeMissing")}
              </span>
            )}
          </div>
          {sys && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
                gap: "8px 12px",
              }}
            >
              <span className="muted">{t("models.ramTotal")}</span>
              <span>{mbToGb(sys.totalMb)} GB</span>
              <span className="muted">{t("models.ramAvailable")}</span>
              <span>{mbToGb(sys.availableMb)} GB</span>
              <span className="muted">{t("models.cpu")}</span>
              <span>
                {t("models.cores", { physical: sys.physicalCores, logical: sys.logicalCores })}
              </span>
              <span className="muted">{t("models.accelerators")}</span>
              <span>{info.data!.providers.map((p) => t(`models.providers.${p}`)).join(" · ")}</span>
              <span className="muted">{t("models.budget")}</span>
              <span>
                {mbToGb(info.data!.budget.budgetMb)} GB ·{" "}
                {t(`models.limitedBy.${info.data!.budget.limitedBy}`)}
              </span>
            </div>
          )}
          <button
            type="button"
            className="btn"
            style={{ alignSelf: "flex-start" }}
            onClick={() => info.refetch()}
          >
            {t("models.recheck")}
          </button>
        </section>
        <section
          style={{ ...card, flex: "1 1 360px", display: "flex", flexDirection: "column", gap: 12 }}
        >
          <h2 style={{ margin: 0, fontSize: 16 }}>{t("models.execution")}</h2>
          <label style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8 }}>
            {t("models.provider")}
            <select
              className="field"
              value={info.data?.executionProvider ?? "cpu"}
              onChange={(e) => run(() => setProvider(e.target.value as ExecutionProvider))}
            >
              <option value="auto">{t("models.providerOptions.auto")}</option>
              <option value="cpu">{t("models.providerOptions.cpu")}</option>
              <option value="direct_ml" disabled={!info.data?.providers.includes("direct_ml")}>
                {t("models.providerOptions.direct_ml")}
              </option>
              <option value="cuda" disabled={!info.data?.providers.includes("cuda")}>
                {t("models.providerOptions.cuda")}
              </option>
              <option value="core_ml" disabled={!info.data?.providers.includes("core_ml")}>
                {t("models.providerOptions.core_ml")}
              </option>
            </select>
          </label>
          <label style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8 }}>
            {t("models.profile")}
            <select
              className="field"
              value={info.data?.resourceProfile ?? "balanced"}
              onChange={(e) => run(() => setProfile(e.target.value as ResourceProfile))}
            >
              <option value="light">{t("models.profiles.light")}</option>
              <option value="balanced">{t("models.profiles.balanced")}</option>
              <option value="performance">{t("models.profiles.performance")}</option>
            </select>
          </label>
          <span className="muted" style={{ fontSize: 13 }}>
            {t("models.providerHint")}
          </span>
        </section>
      </div>
      <section style={{ ...card, display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8 }}>
          <h2 style={{ margin: 0, fontSize: 16 }}>{t("models.models")}</h2>
          <span className="muted" data-testid="models-summary">
            · {t("models.summary", { ready, total: models.length, size: fmtBytes(total) })}
          </span>
          <button
            type="button"
            className="btn btn-primary"
            style={{ marginLeft: "auto" }}
            onClick={() => run(() => commands.verifyModels())}
          >
            {t("models.verifyAll")}
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => run(() => commands.selfTestModels([]))}
          >
            {t("models.testAll")}
          </button>
        </div>
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", minWidth: 900 }}>
            <thead>
              <tr
                style={{
                  color: "var(--pc-text-secondary)",
                  textAlign: "left",
                  fontSize: 12,
                  textTransform: "uppercase",
                  letterSpacing: 0.6,
                }}
              >
                <th style={{ padding: "8px 8px 8px 0" }}>{t("models.col.model")}</th>
                <th style={{ padding: 8 }}>{t("models.col.use")}</th>
                <th style={{ padding: 8 }}>{t("models.col.version")}</th>
                <th style={{ padding: 8 }}>{t("models.col.size")}</th>
                <th style={{ padding: 8 }}>SHA-256</th>
                <th style={{ padding: 8 }}>{t("models.col.state")}</th>
                <th style={{ padding: 8 }}>{t("models.col.test")}</th>
                <th style={{ padding: 8 }}>
                  <span className="visually-hidden">{t("models.col.actions")}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {models.map((m) => (
                <ModelRow key={m.entry.id} m={m} />
              ))}
            </tbody>
          </table>
        </div>
      </section>
      {tested.length > 0 && (
        <section style={{ ...card, display: "flex", flexDirection: "column", gap: 8 }}>
          <h2 style={{ margin: "0 0 4px", fontSize: 16 }}>{t("models.selfTests")}</h2>
          <div
            className="mono"
            style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 12 }}
          >
            {tested.map((m) => (
              <span
                key={m.entry.id}
                style={{ color: m.selfTest!.passed ? "var(--pc-ok)" : "var(--pc-danger)" }}
              >
                {m.selfTest!.passed ? "✓" : "✕"} {m.entry.name}: {m.selfTest!.message} ·{" "}
                {fmtDec(m.selfTest!.avgMs ?? 0, 1)} ms · {fmtDateTime(m.selfTest!.at)}
              </span>
            ))}
          </div>
        </section>
      )}
      <p className="muted" style={{ fontSize: 12 }}>
        {t("models.privacy", { n: fmtInt(models.length) })}
      </p>
    </div>
  );
}
