// Primo avvio in tre passi (desktop-shell, design D8): benvenuto con lingua e tema,
// prima cartella, funzioni AI. Ogni passo si salta con "Lo faccio dopo"; le scelte
// diventano effettive solo confermando il passo, così "Rivedi l'introduzione" non
// cambia nulla finché l'utente non conferma. Alla fine si apre la Libreria.
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, FolderPicker, SegmentedControl, type Option } from "../../components/ui";
import type { Language } from "../../i18n";
import { commands, isTauri, type SystemInfo } from "../../lib/ipc";
import { summarizeModels } from "../../lib/models";
import { useLayout } from "../../store/layout";
import { notify, notifyError } from "../../store/toasts";
import { useGuide } from "../../store/guide";
import type { ThemePref } from "../../theme/tokens";
import "../../components/guide/guide.css";

const DEFAULT_IGNORES = ["**/.thumbnails/**", "**/$RECYCLE.BIN/**"];

/** Il riepilogo del sistema in frasi, non in sigle. */
function SystemSummary({ info }: { info: SystemInfo | undefined }) {
  const { t } = useTranslation();
  if (!info) return <p className="muted">{t("onboarding.checking")}</p>;
  const gb = Math.max(1, Math.round(info.system.totalMb / 1024));
  const gpu = info.providers.some((p) => p !== "cpu");
  return (
    <>
      <p>
        {t("onboarding.summary", {
          gb,
          cores: info.system.logicalCores,
          profile: t(`models.profiles.${info.resourceProfile}`),
        })}
      </p>
      <p>
        {info.onnxRuntime == null
          ? t("onboarding.noRuntime")
          : gpu
            ? t("onboarding.gpu")
            : t("onboarding.cpuOnly")}
      </p>
    </>
  );
}

export function Onboarding({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [language, setLanguage] = useState<Language>(() => useLayout.getState().language);
  const [theme, setTheme] = useState<ThemePref>(() => useLayout.getState().theme);
  const [folder, setFolder] = useState("");
  const [recursive, setRecursive] = useState(true);
  // La verifica del sistema avviene comunque, in background, dal primo passo.
  const system = useQuery({
    queryKey: ["system"],
    queryFn: () => commands.systemInfo(),
    enabled: isTauri(),
  });
  const models = useQuery({
    queryKey: ["models"],
    queryFn: () => commands.listModels(),
    enabled: isTauri(),
  });
  const modelsReady = models.data != null && summarizeModels(models.data).kind === "ready";

  const finish = async () => {
    await commands.completeOnboarding();
    useLayout.getState().openModule({ module: "library" });
    onDone();
  };
  const startDownload = async () => {
    await commands.verifyModels();
    notify({ tone: "ok", message: t("onboarding.downloadStarted") });
  };

  const confirmWelcome = () => {
    useLayout.getState().setLanguage(language);
    useLayout.getState().setTheme(theme);
    setStep(1);
  };
  const confirmFolder = async () => {
    try {
      const id = await commands.addSource({
        path: folder.trim(),
        recursive,
        includeFormats: [],
        ignorePatterns: DEFAULT_IGNORES,
        watch: false,
      });
      const job = await commands.startImport(id, null);
      useGuide.getState().complete("import");
      notify({
        tone: "ok",
        message: t("onboarding.importStarted"),
        action: {
          label: t("onboarding.followImport"),
          run: () => useLayout.getState().openModule({ module: "jobs", target: job }),
        },
      });
      setStep(2);
    } catch (e) {
      notifyError(e);
    }
  };
  const confirmAi = async () => {
    try {
      if (!modelsReady) await startDownload();
      await finish();
    } catch (e) {
      notifyError(e);
    }
  };
  const skipAi = async () => {
    try {
      await finish();
    } catch (e) {
      notifyError(e);
      return;
    }
    if (modelsReady) return;
    // Funzioni AI rimandate: un avviso permette di avviare il download più tardi.
    notify({
      tone: "info",
      duration: null,
      key: "models-later",
      message: t("onboarding.aiOff"),
      action: { label: t("onboarding.download"), run: () => startDownload().catch(notifyError) },
    });
  };

  const languages: Option<Language>[] = [
    { value: "it", label: "Italiano" },
    { value: "en", label: "English" },
  ];
  const themes: Option<ThemePref>[] = [
    { value: "system", label: t("settings.system") },
    { value: "light", label: t("settings.light") },
    { value: "dark", label: t("settings.dark") },
  ];

  return (
    <div className="onboarding" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
      <section className="onboarding-card">
        <h1 id="onboarding-title">{t("onboarding.title")}</h1>
        <p className="visually-hidden" aria-live="polite">
          {t("onboarding.progress", { n: step + 1 })}
        </p>
        <ol className="onboarding-steps">
          {[0, 1, 2].map((i) => (
            <li
              key={i}
              aria-current={i === step ? "step" : undefined}
              data-done={i < step || undefined}
            >
              <span className="onboarding-step-n" aria-hidden="true">
                {i + 1}
              </span>
              {t(`onboarding.steps.${i}`)}
            </li>
          ))}
        </ol>

        {step === 0 && (
          <div className="onboarding-body">
            <h2>{t("onboarding.steps.0")}</h2>
            <p>{t("onboarding.welcome")}</p>
            <div className="onboarding-field">
              <span className="ui-field-label">{t("onboarding.language")}</span>
              <SegmentedControl
                label={t("onboarding.language")}
                value={language}
                options={languages}
                onChange={setLanguage}
              />
            </div>
            <div className="onboarding-field">
              <span className="ui-field-label">{t("onboarding.theme")}</span>
              <SegmentedControl
                label={t("onboarding.theme")}
                value={theme}
                options={themes}
                onChange={setTheme}
              />
            </div>
          </div>
        )}

        {step === 1 && (
          <div className="onboarding-body">
            <h2>{t("onboarding.photosTitle")}</h2>
            <p>{t("onboarding.photosHelp")}</p>
            <FolderPicker
              label={t("onboarding.folder")}
              value={folder}
              onChange={setFolder}
              hint={t("onboarding.folderHint")}
            />
            <label className="onboarding-check">
              <input
                type="checkbox"
                checked={recursive}
                onChange={(e) => setRecursive(e.target.checked)}
              />
              {t("onboarding.recursive")}
            </label>
          </div>
        )}

        {step === 2 && (
          <div className="onboarding-body">
            <h2>{t("onboarding.aiTitle")}</h2>
            <p>{t("onboarding.aiHelp")}</p>
            <SystemSummary info={system.data} />
            {modelsReady && <p>{t("onboarding.modelsReady")}</p>}
          </div>
        )}

        <div className="onboarding-actions">
          {step > 0 && (
            <Button variant="ghost" onClick={() => setStep((s) => s - 1)}>
              {t("onboarding.back")}
            </Button>
          )}
          <span className="onboarding-spacer" />
          <Button onClick={step < 2 ? () => setStep((s) => s + 1) : skipAi}>
            {t("onboarding.later")}
          </Button>
          {step === 0 && (
            <Button variant="primary" onClick={confirmWelcome}>
              {t("onboarding.continue")}
            </Button>
          )}
          {step === 1 && (
            <Button
              variant="primary"
              disabledReason={folder.trim() ? undefined : t("onboarding.continueHint")}
              onClick={confirmFolder}
            >
              {t("onboarding.continue")}
            </Button>
          )}
          {step === 2 && (
            <Button variant="primary" onClick={confirmAi}>
              {modelsReady ? t("onboarding.finish") : t("onboarding.start")}
            </Button>
          )}
        </div>
      </section>
    </div>
  );
}
