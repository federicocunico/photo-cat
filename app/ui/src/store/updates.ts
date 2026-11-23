// Aggiornamenti dell'app (app-updates): ultima release, download in background, installazione.
import { useEffect } from "react";
import { create } from "zustand";
import { commands, events, isTauri, type UpdateInfo } from "../lib/ipc";
import i18n from "../i18n";
import { notify } from "./toasts";
import { useLayout } from "./layout";

export type UpdatePhase = "idle" | "available" | "downloading" | "ready" | "failed";

type State = {
  info: UpdateInfo | null;
  phase: UpdatePhase;
  done: number;
  total: number;
  /** Motivo dell'ultimo download non riuscito. */
  error: string | null;
  /** Notifica chiusa: non ricompare fino al prossimo avvio o al controllo su richiesta. */
  dismissed: boolean;
  /** Dialogo "Aggiorna ora" aperto. */
  confirm: boolean;
  /** Esito dell'ultimo controllo su richiesta, in parole (Impostazioni › Informazioni). */
  lastCheck: string | null;
  checking: boolean;
  check: (manual: boolean) => Promise<void>;
  download: () => Promise<void>;
  cancel: () => void;
  install: () => Promise<void>;
  openPage: () => void;
  dismiss: () => void;
  setConfirm: (open: boolean) => void;
};

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
/** Su Windows l'installer si scarica e si avvia; altrove si apre la pagina della release. */
const canInstall = (info: UpdateInfo | null) => Boolean(info?.installer);

export const useUpdates = create<State>((set, get) => ({
  info: null,
  phase: "idle",
  done: 0,
  total: 0,
  error: null,
  dismissed: false,
  confirm: false,
  lastCheck: null,
  checking: false,
  async check(manual) {
    if (get().checking) return;
    set({ checking: true });
    const t = i18n.t.bind(i18n);
    try {
      const info = await commands.checkUpdate();
      if (!info) {
        const { version } = await commands.appInfo();
        const msg = t("updates.upToDate", { version });
        set({ info: null, phase: "idle", lastCheck: msg });
        if (manual) notify({ tone: "ok", message: msg });
        return;
      }
      const busy = get().phase === "downloading" && get().info?.version === info.version;
      set({
        info,
        phase: busy ? "downloading" : info.downloaded ? "ready" : "available",
        error: null,
        // Il controllo su richiesta mostra sempre la notifica.
        dismissed: manual ? false : get().dismissed,
        lastCheck: t("updates.available", { version: info.version }),
      });
    } catch (e) {
      const msg = t("updates.checkFailed", { reason: errorText(e) });
      set({ lastCheck: msg });
      // Il controllo automatico non disturba: niente avvisi senza connessione.
      if (manual) notify({ tone: "error", message: msg });
    } finally {
      set({ checking: false });
    }
  },
  async download() {
    const { info } = get();
    if (!info) return;
    if (!canInstall(info)) {
      get().openPage();
      return;
    }
    set({ phase: "downloading", done: 0, total: info.installer?.size ?? 0, error: null });
    try {
      await commands.downloadUpdate(info.version);
    } catch (e) {
      set({ phase: "failed", error: errorText(e) });
    }
  },
  cancel() {
    void commands.cancelUpdateDownload();
  },
  async install() {
    const { info } = get();
    if (!info) return;
    try {
      await commands.installUpdate(info.version);
      set({ confirm: false });
    } catch (e) {
      set({ confirm: false, phase: "failed", error: errorText(e) });
    }
  },
  openPage() {
    const url = get().info?.pageUrl;
    if (url)
      void commands
        .openReleasePage(url)
        .catch((e) => notify({ tone: "error", message: errorText(e) }));
  },
  dismiss: () => set({ dismissed: true }),
  setConfirm: (confirm) => set({ confirm }),
}));

/** Eventi del download: avanzamento, installer pronto, download non riuscito o interrotto. */
export function useUpdateEvents() {
  useEffect(() => {
    const offs = [
      events.onUpdateProgress((p) => {
        if (useUpdates.getState().info?.version !== p.version) return;
        useUpdates.setState({ phase: "downloading", done: p.done, total: p.total });
      }),
      events.onUpdateReady((r) => {
        if (useUpdates.getState().info?.version !== r.version) return;
        useUpdates.setState({ phase: "ready", confirm: false });
      }),
      events.onUpdateFailed((f) => {
        if (useUpdates.getState().info?.version !== f.version) return;
        useUpdates.setState(
          f.cancelled
            ? { phase: "available", error: null, dismissed: false }
            : { phase: "failed", error: f.message, dismissed: false },
        );
      }),
    ];
    return () => {
      for (const off of offs) void off.then((f) => f());
    };
  }, []);
}

/** Ritardo del controllo all'avvio: prima la libreria, poi la rete. */
export const STARTUP_DELAY_MS = 5000;

/** Controllo automatico all'avvio, se attivo nelle preferenze (Impostazioni › Informazioni). */
export function useStartupUpdateCheck(enabled: boolean) {
  useEffect(() => {
    if (!enabled || !isTauri() || !useLayout.getState().checkUpdatesOnStart) return;
    const timer = setTimeout(() => void useUpdates.getState().check(false), STARTUP_DELAY_MS);
    return () => clearTimeout(timer);
  }, [enabled]);
}
