// Impostazioni a sezioni (desktop-shell, design D9): sotto-navigazione, ricerca che porta
// alla sezione ed evidenzia il campo, salvataggio immediato delle preferenze e barra
// "Ripristina / Applica" per ciò che ricalcola i dati della libreria. Modelli AI e
// Diagnostica sono sezioni: si aprono con `openModule({ module: "settings", section })`.
import { QueryView } from "../../components/feedback/QueryView";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type Settings } from "../../lib/ipc";
import { useLayout, type SettingsSection } from "../../store/layout";
import { PageHeader } from "../../components/ui";
import { Icon } from "../../components/icons";
import { ModelsPage } from "../models/ModelsPage";
import { DiagnosticsPage } from "../diagnostics/DiagnosticsPage";
import { AdvancedSection } from "./AdvancedSection";
import { AiSection } from "./AiSection";
import { ApplyBar } from "./ApplyBar";
import { ExternalAppsSection } from "./ExternalAppsSection";
import { GeneralSection, LibrarySection } from "./GeneralSection";
import { Setting, SettingsProvider } from "./settingsState";
import { useSettings } from "./settingsContext";
import "./settings.css";

const SECTIONS: SettingsSection[] = [
  "general",
  "library",
  "ai",
  "models",
  "apps",
  "diagnostics",
  "advanced",
];

/** Voci cercabili: id del campo, sezione e chiave dell'etichetta. */
const SEARCH: { id: string; section: SettingsSection; label: string }[] = [
  { id: "theme", section: "general", label: "settings.theme" },
  { id: "language", section: "general", label: "settings.language" },
  { id: "doubleClick", section: "general", label: "externalApps.doubleClick" },
  { id: "dataDir", section: "general", label: "settings.path" },
  { id: "ignoredFolders", section: "library", label: "externalApps.ignoredFolders" },
  { id: "importFolders", section: "library", label: "settings.importFolders" },
  { id: "faces", section: "ai", label: "settings.faces" },
  { id: "autoModels", section: "ai", label: "settings.autoModels" },
  { id: "sensitivity", section: "ai", label: "settings.sensitivity.title" },
  { id: "sensitivity", section: "ai", label: "thresholds.persons" },
  { id: "sensitivity", section: "ai", label: "thresholds.faces" },
  { id: "sensitivity", section: "ai", label: "thresholds.faceSharpness" },
  { id: "sensitivity", section: "ai", label: "thresholds.similarity" },
  { id: "sensitivity", section: "ai", label: "similar.threshold" },
  { id: "faceSize", section: "ai", label: "settings.faceSizes.label" },
  { id: "groups", section: "ai", label: "thresholds.groups" },
  { id: "taxonomy", section: "ai", label: "settings.taxonomy" },
  { id: "ramLimit", section: "models", label: "settings.ram" },
  { id: "section", section: "models", label: "models.profile" },
  { id: "section", section: "models", label: "models.provider" },
  { id: "section", section: "models", label: "models.models" },
  { id: "externalApps", section: "apps", label: "externalApps.title" },
  { id: "logLevel", section: "diagnostics", label: "settings.log" },
  { id: "section", section: "diagnostics", label: "diag.exportBundle" },
  { id: "rawThresholds", section: "advanced", label: "settings.rawThresholds" },
  { id: "rawThresholds", section: "advanced", label: "settings.threshold" },
  { id: "purgeFaces", section: "advanced", label: "settings.purge" },
];

const norm = (s: string) => s.toLocaleLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");

function RamLimit() {
  const { t } = useTranslation();
  const { saved, autosave } = useSettings();
  const id = useId();
  return (
    <Setting id="ramLimit" label={t("settings.ram")} labelFor={id} hint={t("settings.ramHint")}>
      <input
        id={id}
        className="field"
        type="number"
        min={512}
        step={256}
        defaultValue={saved.resources.ram_limit_mb ?? ""}
        onChange={(e) => {
          const ram_limit_mb = e.target.value ? Number(e.target.value) : null;
          autosave("ramLimit", (c) => ({ ...c, resources: { ...c.resources, ram_limit_mb } }));
        }}
      />
    </Setting>
  );
}

function SectionContent({ section }: { section: SettingsSection }) {
  switch (section) {
    case "general":
      return <GeneralSection />;
    case "library":
      return <LibrarySection />;
    case "ai":
      return <AiSection />;
    case "models":
      return (
        <>
          <ModelsPage />
          <RamLimit />
        </>
      );
    case "apps":
      return <ExternalAppsSection />;
    case "diagnostics":
      return <DiagnosticsPage />;
    case "advanced":
      return <AdvancedSection />;
  }
}

/** Le sezioni con una schermata propria (Modelli, Diagnostica) hanno già il titolo. */
const OWN_TITLE: SettingsSection[] = ["models", "diagnostics"];

function SettingsLayout() {
  const { t } = useTranslation();
  const section = useLayout((s) => s.settingsSection);
  const setSection = useLayout((s) => s.setSettingsSection);
  const { highlight, setHighlight } = useSettings();
  const [query, setQuery] = useState("");
  const searchId = useId();
  const results = useMemo(() => {
    const q = norm(query.trim());
    if (!q) return [];
    const seen = new Set<string>();
    return SEARCH.map((e) => ({ ...e, text: t(e.label) }))
      .filter(
        (e) => norm(e.text).includes(q) || norm(t(`settings.sections.${e.section}`)).includes(q),
      )
      .filter((e) => {
        const k = `${e.section}/${e.text}`;
        if (seen.has(k)) return false;
        seen.add(k);
        return true;
      });
  }, [query, t]);

  // Dalla ricerca: porta il campo in vista, gli dà il focus e lo evidenzia per un momento.
  useEffect(() => {
    if (!highlight) return;
    const el = document.querySelector<HTMLElement>(`[data-setting="${highlight}"]`);
    el?.scrollIntoView?.({ block: "center" });
    el?.querySelector<HTMLElement>("input, select, textarea, button, [tabindex]")?.focus();
    const timer = setTimeout(() => setHighlight(null), 2500);
    return () => clearTimeout(timer);
  }, [highlight, section, setHighlight]);

  const go = (e: (typeof SEARCH)[number]) => {
    setSection(e.section);
    setQuery("");
    setHighlight(e.id === "section" ? null : e.id);
  };

  return (
    <div className="settings-page">
      <PageHeader title={t("settings.title")} />
      <div className="settings-body">
        <aside className="settings-aside">
          <label htmlFor={searchId} className="visually-hidden">
            {t("settings.search")}
          </label>
          <div className="settings-search">
            <Icon.Search size={16} aria-hidden="true" />
            <input
              id={searchId}
              type="search"
              className="field"
              placeholder={t("settings.search")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && results[0]) go(results[0]);
                if (e.key === "Escape") setQuery("");
              }}
            />
          </div>
          {query.trim() ? (
            <div className="settings-results" aria-live="polite">
              {results.length === 0 ? (
                <p className="muted">{t("settings.noResults")}</p>
              ) : (
                <ul>
                  {results.map((r) => (
                    <li key={`${r.section}/${r.label}`}>
                      <button type="button" className="settings-result" onClick={() => go(r)}>
                        <span>{r.text}</span>
                        <span className="muted">{t(`settings.sections.${r.section}`)}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ) : (
            <nav aria-label={t("settings.sections.label")}>
              <ul className="settings-nav">
                {SECTIONS.map((s) => (
                  <li key={s}>
                    <button
                      type="button"
                      className="settings-nav-item"
                      aria-current={s === section ? "page" : undefined}
                      onClick={() => setSection(s)}
                    >
                      {t(`settings.sections.${s}`)}
                    </button>
                  </li>
                ))}
              </ul>
            </nav>
          )}
        </aside>
        <div className="settings-main">
          <section className="settings-section" aria-labelledby="settings-section-title">
            <h2
              id="settings-section-title"
              className={OWN_TITLE.includes(section) ? "visually-hidden" : undefined}
            >
              {t(`settings.sections.${section}`)}
            </h2>
            <SectionContent section={section} />
          </section>
          <ApplyBar />
        </div>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const config = useQuery({
    queryKey: ["config"],
    queryFn: commands.getConfig,
    enabled: isTauri(),
  });
  const taxonomy = useQuery({
    queryKey: ["taxonomy"],
    queryFn: commands.getTaxonomy,
    enabled: isTauri(),
  });
  return (
    <QueryView query={config} enabled={isTauri()} isEmpty={() => false}>
      {(cfg) => (
        <SettingsProvider config={cfg as Settings} taxonomy={taxonomy.data}>
          <SettingsLayout />
        </SettingsProvider>
      )}
    </QueryView>
  );
}
