import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
export function Onboarding({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [source, setSource] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const system = useQuery({
    queryKey: ["system"],
    queryFn: commands.systemInfo,
    enabled: isTauri(),
  });
  const models = useQuery({
    queryKey: ["models"],
    queryFn: commands.listModels,
    enabled: isTauri() && step === 2,
    refetchInterval: 1000,
  });
  const run = async (fn: () => Promise<void>) => {
    setError("");
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const finish = async () => {
    await commands.completeOnboarding();
    onDone();
  };
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("onboarding.title")}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 50,
        background: "var(--pc-bg)",
        display: "grid",
        placeItems: "center",
        overflow: "auto",
      }}
    >
      <section
        className="feature-card"
        style={{ width: "min(720px,90vw)", alignItems: "stretch", padding: 32 }}
      >
        <h1>{t("onboarding.title")}</h1>
        <p>
          {step + 1} / 4 · {t(`onboarding.steps.${step}`)}
        </p>
        {error && <p role="alert">{error}</p>}
        {step === 0 && <p>{t("onboarding.welcome")}</p>}
        {step === 1 && (
          <>
            <p>{t("onboarding.system")}</p>
            <p>
              {system.data
                ? `${Math.round(system.data.system.totalMb / 1024)} GB RAM · ${system.data.system.logicalCores} CPU · ${Math.round(system.data.budget.budgetMb)} MB`
                : t("settings.loading")}
            </p>
            <p>{system.data?.onnxRuntime ?? t("onboarding.noRuntime")}</p>
          </>
        )}
        {step === 2 && (
          <>
            <p>{t("onboarding.models")}</p>
            <p>
              {models.data?.filter((m) => m.state.state === "verified").length ?? 0} /{" "}
              {models.data?.length ?? 7}
            </p>
            <button
              className="btn"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await commands.verifyModels();
                })
              }
            >
              {t("onboarding.prepareModels")}
            </button>
          </>
        )}
        {step === 3 && (
          <>
            <p>{t("onboarding.source")}</p>
            <input
              className="field"
              aria-label={t("onboarding.folder")}
              value={source}
              onChange={(e) => setSource(e.target.value)}
            />
            <button
              className="btn"
              onClick={() =>
                void run(async () => {
                  const { open } = await import("@tauri-apps/plugin-dialog");
                  const p = await open({ directory: true, multiple: false });
                  if (typeof p === "string") setSource(p);
                })
              }
            >
              {t("organize.browse")}
            </button>
            <button
              className="btn"
              disabled={busy || !source.trim()}
              onClick={() =>
                void run(async () => {
                  const id = await commands.addSource({
                    path: source,
                    recursive: true,
                    includeFormats: [],
                    ignorePatterns: ["**/.thumbnails/**", "**/$RECYCLE.BIN/**"],
                    watch: false,
                  });
                  await commands.startImport(id);
                  await finish();
                  useLayout.getState().setModule("jobs");
                })
              }
            >
              {t("onboarding.import")}
            </button>
          </>
        )}
        <div className="feature-actions">
          {step > 0 && (
            <button className="btn" onClick={() => setStep((s) => s - 1)}>
              {t("onboarding.back")}
            </button>
          )}
          {step < 3 ? (
            <button className="btn" onClick={() => setStep((s) => s + 1)}>
              {t("onboarding.next")}
            </button>
          ) : (
            <button className="btn" disabled={busy} onClick={() => void run(finish)}>
              {t("onboarding.later")}
            </button>
          )}
          <button className="btn" disabled={busy} onClick={() => void run(finish)}>
            {t("onboarding.skip")}
          </button>
        </div>
      </section>
    </div>
  );
}
