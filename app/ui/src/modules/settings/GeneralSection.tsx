// Impostazioni › Generale e › Libreria e importazione.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, SegmentedControl, type Option } from "../../components/ui";
import type { Language } from "../../i18n";
import { commands, isTauri, type Settings } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { notify, notifyError } from "../../store/toasts";
import type { ThemePref } from "../../theme/tokens";
import { Setting } from "./settingsState";
import { useSettings } from "./settingsContext";

export function GeneralSection() {
  const { t } = useTranslation();
  const { saved, draft, setDraft, autosave } = useSettings();
  const info = useQuery({ queryKey: ["app-info"], queryFn: commands.appInfo, enabled: isTauri() });
  const dcId = useId();
  const pathId = useId();
  const themes: Option<ThemePref>[] = [
    { value: "system", label: t("settings.system") },
    { value: "light", label: t("settings.light") },
    { value: "dark", label: t("settings.dark") },
  ];
  const languages: Option<Language>[] = [
    { value: "it", label: "Italiano" },
    { value: "en", label: "English" },
  ];
  const apps = saved.external_apps ?? [];
  return (
    <>
      <Setting id="theme" label={t("settings.theme")}>
        <SegmentedControl
          label={t("settings.theme")}
          value={(saved.ui.theme ?? "system") as ThemePref}
          options={themes}
          onChange={(theme) => {
            useLayout.getState().setTheme(theme);
            autosave("theme", (c) => ({ ...c, ui: { ...c.ui, theme } }));
          }}
        />
      </Setting>
      <Setting id="language" label={t("settings.language")}>
        <SegmentedControl
          label={t("settings.language")}
          value={(saved.ui.language ?? "it") as Language}
          options={languages}
          onChange={(language) => {
            useLayout.getState().setLanguage(language);
            autosave("language", (c) => ({ ...c, ui: { ...c.ui, language } }));
          }}
        />
      </Setting>
      <Setting id="doubleClick" label={t("externalApps.doubleClick")} labelFor={dcId}>
        <select
          id={dcId}
          className="field"
          value={saved.ui.double_click ?? "system"}
          onChange={(e) => {
            const double_click = e.target.value;
            autosave("doubleClick", (c) => ({ ...c, ui: { ...c.ui, double_click } }));
          }}
        >
          <option value="system">{t("externalApps.system")}</option>
          <option value="viewer">{t("externalApps.viewer")}</option>
          {apps.map((a) => (
            <option key={a.id} value={a.id}>
              {a.name || a.id}
            </option>
          ))}
          <option value="none">{t("externalApps.none")}</option>
        </select>
      </Setting>
      <Setting
        id="dataDir"
        label={t("settings.path")}
        labelFor={pathId}
        hint={
          <>
            {t("settings.currentPath")}: <span className="mono">{info.data?.dataDir}</span>
            <br />
            {t("settings.pathHint")}
          </>
        }
      >
        <input
          id={pathId}
          className="field settings-wide"
          value={draft.data_dir ?? ""}
          spellCheck={false}
          onChange={(e) => setDraft({ ...draft, data_dir: e.target.value || null })}
        />
      </Setting>
    </>
  );
}

/** Cartelle da ignorare: si salvano dopo la pausa nella digitazione. */
function IgnoredFolders() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { saved, autosave } = useSettings();
  const id = useId();
  const [text, setText] = useState((saved.ai.ignored_folder_names ?? []).join("\n"));
  const offerRecompute = (next: Settings, prev: Settings) => {
    const clean = (c: Settings) =>
      (c.ai.ignored_folder_names ?? []).map((x) => x.trim()).filter(Boolean);
    if (JSON.stringify(clean(next)) === JSON.stringify(clean(prev))) return;
    // Il luogo dedotto dalle cartelle cambia: si propone il ricalcolo, senza bloccare.
    notify({
      key: "recompute-places",
      message: t("settings.recomputeAsk"),
      action: {
        label: t("settings.recompute"),
        run: async () => {
          try {
            const n = await commands.recomputePlaces();
            await qc.invalidateQueries({ queryKey: ["facets"] });
            await qc.invalidateQueries({ queryKey: ["library"] });
            notify({ tone: "ok", message: t("externalApps.recomputed", { count: n }) });
          } catch (e) {
            notifyError(e);
          }
        },
      },
    });
  };
  return (
    <Setting
      id="ignoredFolders"
      label={t("externalApps.ignoredFolders")}
      labelFor={id}
      hint={t("externalApps.ignoredHint")}
    >
      <textarea
        id={id}
        className="field settings-wide"
        rows={4}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          const ignored_folder_names = e.target.value.split("\n");
          autosave(
            "ignoredFolders",
            (c) => ({ ...c, ai: { ...c.ai, ignored_folder_names } }),
            offerRecompute,
          );
        }}
      />
    </Setting>
  );
}

export function LibrarySection() {
  const { t } = useTranslation();
  return (
    <>
      <IgnoredFolders />
      <Setting id="importFolders" label={t("settings.importFolders")}>
        <p className="muted settings-inline-note">{t("settings.importFoldersHint")}</p>
        <Button onClick={() => useLayout.getState().openModule({ module: "import" })}>
          {t("settings.openImport")}
        </Button>
      </Setting>
    </>
  );
}
