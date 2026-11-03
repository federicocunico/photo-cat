import { useEffect, useState } from "react";
import { AppShell, CrashBanner, UiRecoveredBanner } from "./components/shell/AppShell";
import { StatusBar } from "./components/shell/StatusBar";
import { setLanguage } from "./i18n";
import { commands, events, isTauri, type CrashInfo, type UiRecovery } from "./lib/ipc";
import { startLongTaskReporter } from "./lib/longTasks";
import { DiagnosticsPage } from "./modules/diagnostics/DiagnosticsPage";
import { ImportPage } from "./modules/import/ImportPage";
import { JobsPage } from "./modules/jobs/JobsPage";
import { LibraryPage } from "./modules/library/LibraryPage";
import { LibraryStatus } from "./modules/library/LibraryStatus";
import { ModelsPage } from "./modules/models/ModelsPage";
import { PeopleModule } from "./modules/people/PeopleModule";
import { CatalogsPage } from "./modules/catalogs/CatalogsPage";
import { OrganizePage } from "./modules/organize/OrganizePage";
import { SettingsPage } from "./modules/settings/SettingsPage";
import { TagsPage } from "./modules/tags/TagsPage";
import { SequencePage, SequenceStatus } from "./modules/sequence/SequencePage";
import { ActiveSetStatus, TagsStatus } from "./modules/tags/TagStatus";
import { Onboarding } from "./modules/onboarding/Onboarding";
import { Placeholder } from "./modules/Placeholder";
import { useLayout, type ModuleId } from "./store/layout";
import { useLiveEvents, useTrimCaches } from "./store/live";
import { useImportRefresh } from "./modules/import/sourceActions";
import { ToastHost } from "./components/feedback/ToastHost";
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
      return <PeopleModule />;
    case "catalogs":
      return <CatalogsPage />;
    case "tags":
      return <TagsPage />;
    case "sequence":
      return <SequencePage />;
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

export type StartupData = { onboarding: boolean; info: { previousCrash: CrashInfo | null } };

export function ReadyApp({ startup }: { startup: StartupData }) {
  const module = useLayout((s) => s.module);
  const theme = useLayout((s) => s.theme);
  const language = useLayout((s) => s.language);
  const [crash, setCrash] = useState<CrashInfo | null>(startup.info.previousCrash);
  const [recovery, setRecovery] = useState<UiRecovery | null>(null);
  const [onboarding, setOnboarding] = useState(startup.onboarding);
  useLiveEvents();
  useTrimCaches();
  useImportRefresh();
  useEffect(() => startLongTaskReporter(), []);
  useEffect(() => {
    const check = () =>
      void commands
        .takeUiRecovery()
        .then((r) => r && setRecovery(r))
        .catch(() => {});
    if (isTauri()) check();
    const off = events.onUiRecovered(check);
    return () => {
      void off.then((f) => f());
    };
  }, []);
  useEffect(() => {
    // Solo sviluppo: VITE_START_MODULE=jobs apre direttamente un modulo.
    const start = import.meta.env.VITE_START_MODULE as ModuleId | undefined;
    if (import.meta.env.DEV && start) useLayout.getState().setModule(start);
  }, []);
  useEffect(() => applyTheme(theme), [theme]);
  useEffect(() => setLanguage(language), [language]);
  const dismiss = () => {
    setCrash(null);
    void commands.dismissCrashNotice();
  };
  if (onboarding)
    return (
      <>
        <Onboarding onDone={() => setOnboarding(false)} />
        <ToastHost />
      </>
    );
  return (
    <AppShell
      banner={
        <>
          {crash && <CrashBanner crash={crash} onDismiss={dismiss} />}
          {recovery && <UiRecoveredBanner info={recovery} onDismiss={() => setRecovery(null)} />}
        </>
      }
    >
      <ModuleView module={module} />
      <ToastHost />
      <StatusBar>
        {module === "library" && (
          <>
            <LibraryStatus />
            <ActiveSetStatus />
          </>
        )}
        {module === "tags" && <TagsStatus />}
        {module === "sequence" && (
          <>
            <SequenceStatus />
            <ActiveSetStatus />
          </>
        )}
      </StatusBar>
    </AppShell>
  );
}
