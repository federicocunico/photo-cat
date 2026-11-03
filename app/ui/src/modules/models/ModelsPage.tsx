import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type ExecutionProvider,
  type ModelState,
  type ModelStatus,
  type ResourceProfile,
} from "../../lib/ipc";
import { fmtBytes, fmtDateTime, fmtDec, fmtInt, mbToGb } from "../../lib/format";
import { Icon } from "../../components/icons";
import { useModelStatuses } from "../../lib/models";
import "./models.css";

function stateBadge(s: ModelState) {
  switch (s.state) {
    case "verified":
      return "models-badge--ok";
    case "downloading":
    case "update_available":
      return "models-badge--accent";
    case "missing":
      return "models-badge--warn";
    default:
      return "models-badge--danger";
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

const useModels = useModelStatuses;

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
    <tr className="models-tr" data-testid={`model-${m.entry.id}`}>
      <td className="models-td--first">{m.entry.name}</td>
      <td className="models-td models-td--soft">{t(`models.roles.${m.entry.role}`)}</td>
      <td className="mono models-td models-td--small">{m.entry.version}</td>
      <td className="mono models-td models-td--small">{fmtBytes(m.entry.sizeBytes)}</td>
      <td className="mono muted models-td models-td--small" title={m.entry.sha256}>
        {m.entry.sha256.slice(0, 4)}…{m.entry.sha256.slice(-4)}
      </td>
      <td className="models-td">
        <div className="models-stack">
          <span className={`models-badge ${stateBadge(s)}`}>{label}</span>
          {pct !== null && (
            <>
              <div className="bar models-bar">
                <span style={{ width: `${pct}%` }} />
              </div>
              {s.state === "downloading" && (
                <span className="mono muted models-small">
                  {fmtBytes(s.done_bytes)} / {fmtBytes(s.total_bytes)} ·{" "}
                  {fmtBytes(s.bytes_per_second ?? 0)}/s
                </span>
              )}
            </>
          )}
          {(s.state === "error" || s.state === "self_test_failed") && (
            <span className="muted models-small">{s.message}</span>
          )}
          {error && (
            <span role="alert" className="models-danger-small">
              {error}
            </span>
          )}
        </div>
      </td>
      <td className="muted models-td models-td--small">
        {m.selfTest
          ? `${m.selfTest.passed ? t("models.passed") : t("models.failed")} · ${fmtDec(m.selfTest.avgMs ?? 0, 1)} ms`
          : "—"}
      </td>
      <td className="models-td">
        <div className="models-actions">
          {s.state !== "verified" && s.state !== "downloading" && (
            <BusyButton
              type="button"
              className="btn"
              onClick={() => run(() => commands.downloadModel(m.entry.id))}
            >
              {t("models.download")}
            </BusyButton>
          )}
          <BusyButton
            type="button"
            className="btn btn-ghost"
            disabled={s.state !== "verified" && s.state !== "self_test_failed"}
            onClick={() => run(() => commands.selfTestModels([m.entry.id]))}
          >
            {t("models.test")}
          </BusyButton>
          <BusyButton
            type="button"
            className="btn btn-ghost"
            disabled={s.state === "downloading"}
            onClick={install}
          >
            {t("models.installFile")}
          </BusyButton>
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
    <QueryView query={info} enabled={isTauri()} isEmpty={() => false}>
      {() => (
        <div className="models-page">
          <h2 className="models-title">{t("models.title")}</h2>
          {error && (
            <p role="alert" className="models-error">
              {error}
            </p>
          )}
          <div className="models-cards">
            <section className="models-card models-card--half">
              <div className="models-row">
                <h2 className="models-h">{t("models.system")}</h2>
                {info.data && (
                  <span
                    className={`models-badge models-push ${info.data.onnxRuntime ? "models-badge--ok" : "models-badge--danger"}`}
                  >
                    {info.data.onnxRuntime ? t("models.runtimeOk") : t("models.runtimeMissing")}
                  </span>
                )}
              </div>
              {sys && (
                <div className="models-sysgrid">
                  <span className="muted">{t("models.ramTotal")}</span>
                  <span>{mbToGb(sys.totalMb)} GB</span>
                  <span className="muted">{t("models.ramAvailable")}</span>
                  <span>{mbToGb(sys.availableMb)} GB</span>
                  <span className="muted">{t("models.cpu")}</span>
                  <span>
                    {t("models.cores", { physical: sys.physicalCores, logical: sys.logicalCores })}
                  </span>
                  <span className="muted">{t("models.accelerators")}</span>
                  <span>
                    {info.data!.providers.map((p) => t(`models.providers.${p}`)).join(" · ")}
                  </span>
                  <span className="muted">{t("models.budget")}</span>
                  <span>
                    {mbToGb(info.data!.budget.budgetMb)} GB ·{" "}
                    {t(`models.limitedBy.${info.data!.budget.limitedBy}`)}
                  </span>
                </div>
              )}
              <BusyButton type="button" className="btn models-start" onClick={() => info.refetch()}>
                {t("models.recheck")}
              </BusyButton>
            </section>
            <section className="models-card models-card--half">
              <h2 className="models-h">{t("models.execution")}</h2>
              <label className="models-row models-row--wrap">
                {t("models.provider")}
                <select
                  className="field"
                  value={info.data?.executionProvider ?? "cpu"}
                  onChange={(e) => run(() => setProvider(e.target.value as ExecutionProvider))}
                >
                  <option value="auto">{t("models.providerOptions.auto")}</option>
                  <option value="cpu">{t("models.providerOptions.cpu")}</option>
                  {(["direct_ml", "cuda", "core_ml"] as const).map((p) => {
                    const reason = info.data?.unavailableProviders.find(
                      (u) => u.provider === p,
                    )?.reason;
                    return (
                      <option key={p} value={p} disabled={!info.data?.providers.includes(p)}>
                        {t(`models.providerOptions.${p}`)}
                        {reason &&
                          ` — ${t(`models.unavailable.${reason}`, {
                            platform: t(`models.unavailable.platforms.${p}`),
                          })}`}
                      </option>
                    );
                  })}
                </select>
              </label>
              <label className="models-row models-row--wrap">
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
              <span className="muted models-hint">{t("models.providerHint")}</span>
            </section>
          </div>
          <section className="models-card">
            <div className="models-row models-row--wrap">
              <h2 className="models-h">{t("models.models")}</h2>
              <span className="muted" data-testid="models-summary">
                · {t("models.summary", { ready, total: models.length, size: fmtBytes(total) })}
              </span>
              <BusyButton
                type="button"
                className="btn btn-primary models-push"
                onClick={() => run(() => commands.verifyModels())}
              >
                {t("models.verifyAll")}
              </BusyButton>
              <BusyButton
                type="button"
                className="btn"
                onClick={() => run(() => commands.selfTestModels([]))}
              >
                {t("models.testAll")}
              </BusyButton>
            </div>
            <div className="models-scroll">
              <table className="models-table">
                <thead>
                  <tr className="models-thead">
                    <th>{t("models.col.model")}</th>
                    <th>{t("models.col.use")}</th>
                    <th>{t("models.col.version")}</th>
                    <th>{t("models.col.size")}</th>
                    <th>SHA-256</th>
                    <th>{t("models.col.state")}</th>
                    <th>{t("models.col.test")}</th>
                    <th>
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
            <section className="models-card models-card--tight">
              <h2 className="models-h-tests">{t("models.selfTests")}</h2>
              <div className="mono models-tests">
                {tested.map((m) => (
                  <span
                    key={m.entry.id}
                    className={m.selfTest!.passed ? "models-test--ok" : "models-test--fail"}
                  >
                    {m.selfTest!.passed ? <Icon.Check size={14} /> : <Icon.Close size={14} />}{" "}
                    {m.entry.name}: {m.selfTest!.message} · {fmtDec(m.selfTest!.avgMs ?? 0, 1)} ms ·{" "}
                    {fmtDateTime(m.selfTest!.at)}
                  </span>
                ))}
              </div>
            </section>
          )}
          <p className="muted models-small">{t("models.privacy", { n: fmtInt(models.length) })}</p>
        </div>
      )}
    </QueryView>
  );
}
