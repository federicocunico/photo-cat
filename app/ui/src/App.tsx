import { useEffect, useState } from "react";
import { AppShell, CrashBanner } from "./components/shell/AppShell";
import { StatusBar } from "./components/shell/StatusBar";
import { setLanguage } from "./i18n";
import { commands, isTauri, type CrashInfo, type Settings } from "./lib/ipc";
import { DiagnosticsPage } from "./modules/diagnostics/DiagnosticsPage";
import { ImportPage } from "./modules/import/ImportPage";
import { JobsPage } from "./modules/jobs/JobsPage";
import { LibraryPage } from "./modules/library/LibraryPage";
import { LibraryStatus } from "./modules/library/LibraryStatus";
import { ModelsPage } from "./modules/models/ModelsPage";
import { PeoplePage } from "./modules/people/PeoplePage";
import { CatalogsPage } from "./modules/catalogs/CatalogsPage";
import { OrganizePage } from "./modules/organize/OrganizePage";
import { SettingsPage } from "./modules/settings/SettingsPage";
import { Onboarding } from "./modules/onboarding/Onboarding";
import { Placeholder } from "./modules/Placeholder";
import { useLayout, type ModuleId } from "./store/layout";
import { useLiveEvents } from "./store/live";
import { applyTheme } from "./theme/tokens";

function ModuleView({ module }: { module: ModuleId }) {
  switch (module) {
    case "library":
      return <LibraryPage />;
    case "import":
      return <ImportPage />;
    case "jobs":
      return <JobsPage />;
    case "models":
      return <ModelsPage />;
    case "people":
      return <PeoplePage />;
    case "catalogs":
      return <CatalogsPage />;
    case "organize":
      return <OrganizePage />;
    case "settings":
      return <SettingsPage />;
    case "diagnostics":
      return <DiagnosticsPage />;
    default:
      return <Placeholder module={module} />;
  }
}

export function App() {
  const module = useLayout((s) => s.module);
  const theme = useLayout((s) => s.theme);
  const language = useLayout((s) => s.language);
  const [crash, setCrash] = useState<CrashInfo | null>(null);
  const [onboarding, setOnboarding] = useState(false);
  useLiveEvents();
  useEffect(() => {
    // Solo sviluppo: VITE_START_MODULE=jobs apre direttamente un modulo.
    const start = import.meta.env.VITE_START_MODULE as ModuleId | undefined;
    if (import.meta.env.DEV && start) useLayout.getState().setModule(start);
  }, []);
  useEffect(() => applyTheme(theme), [theme]);
  useEffect(() => setLanguage(language), [language]);
  useEffect(() => {
    if (isTauri()) {
      void commands.onboardingDone().then((done) => setOnboarding(!done));
      void commands.getConfig().then((c) => {
        const cfg = c as Settings;
        useLayout.getState().setTheme(cfg.ui.theme);
        useLayout.getState().setLanguage(cfg.ui.language);
      });
    }
    if (isTauri())
      commands
        .appInfo()
        .then((i) => setCrash(i.previousCrash))
        .catch(() => {});
  }, []);
  const dismiss = () => {
    setCrash(null);
    void commands.dismissCrashNotice();
  };
  return (
    <AppShell banner={crash && <CrashBanner crash={crash} onDismiss={dismiss} />}>
      <ModuleView module={module} />
      {onboarding && <Onboarding onDone={() => setOnboarding(false)} />}
      <StatusBar>{module === "library" && <LibraryStatus />}</StatusBar>
    </AppShell>
  );
}
