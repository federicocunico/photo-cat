import { useEffect, useState, type ComponentType } from "react";
import { BootScreen } from "./components/feedback/BootScreen";
import { commands, events, isTauri, type BootStatus, type Settings } from "./lib/ipc";
import { restoreUiPrefs } from "./lib/uiPrefs";
import type { StartupData } from "./ReadyApp";

const initial: BootStatus = { phase: "opening", ready: false, error: null, logsPath: "" };
export function App() {
  const [status, setStatus] = useState(initial);
  const [loaded, setLoaded] = useState<{
    View: ComponentType<{ startup: StartupData }>;
    data: StartupData;
  } | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let cancelled = false,
      preparing = false;
    const update = async (next: BootStatus) => {
      if (cancelled || preparing) return;
      setStatus(next);
      if (!next.ready) return;
      preparing = true;
      setStatus({ ...next, ready: false, phase: "configuration" });
      try {
        // Keep the preference restore before importing any persisted stores.
        await restoreUiPrefs();
        const [data, { ReadyApp }, { useLayout }] = await Promise.all([
          isTauri()
            ? Promise.all([commands.onboardingDone(), commands.getConfig(), commands.appInfo()])
            : Promise.resolve([true, null, { previousCrash: null }] as const),
          import("./ReadyApp"),
          import("./store/layout"),
        ]);
        const [done, config, info] = data;
        if (cancelled) return;
        if (config) {
          const cfg = config as Settings;
          useLayout.getState().setTheme(cfg.ui.theme);
          useLayout.getState().setLanguage(cfg.ui.language);
        }
        setLoaded({ View: ReadyApp, data: { onboarding: !done, info } });
      } catch (e) {
        preparing = false;
        if (!cancelled)
          setStatus({
            ...next,
            ready: false,
            error: (e as { detail?: string })?.detail ?? String(e),
          });
      }
    };
    const read = () => {
      if (cancelled || preparing) return;
      if (isTauri())
        void commands
          .bootStatus()
          .then(update)
          .catch((e: unknown) => {
            if (!cancelled) setStatus((s) => ({ ...s, error: String(e) }));
          });
      else void update({ ...initial, ready: true });
    };
    const subscriptions = [
      events.onBootPhase((s) => void update(s)),
      events.onBootReady((s) => void update(s)),
    ];
    read();
    const timer = setInterval(read, 1000);
    return () => {
      cancelled = true;
      clearInterval(timer);
      subscriptions.forEach((p) => void p.then((off) => off()));
    };
  }, [attempt]);
  if (loaded) return <loaded.View startup={loaded.data} />;
  return (
    <BootScreen
      status={status}
      retry={async () => {
        await commands.bootRetry();
        setAttempt((a) => a + 1);
      }}
      choose={async () => {
        await commands.bootChooseFolder();
        setAttempt((a) => a + 1);
      }}
    />
  );
}
