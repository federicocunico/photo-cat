// Stato della fotocamera Fujifilm per il modulo Varianti RAW. `useCameraStatus` è passivo (comando
// leggero più eventi) e serve alla barra della sessione; `useCameraMonitor` aggiunge la verifica
// completa, il registro e il controllo periodico che servono al dialogo di sviluppo.
import { useCallback, useEffect, useRef, useState } from "react";
import {
  commands,
  events,
  type CameraDiagnostics,
  type CameraLogLine,
  type CameraStatus,
} from "../../lib/ipc";
import { appendLog, nextPollDelay } from "./develop";

/** Stato della fotocamera, aggiornato dagli eventi del backend. `undefined` finché non è noto. */
export function useCameraStatus(): CameraStatus | undefined {
  const [status, setStatus] = useState<CameraStatus>();
  useEffect(() => {
    let alive = true;
    commands
      .rawReviewCameraStatus()
      .then((s) => alive && setStatus((current) => current ?? s))
      .catch(() => {});
    const un = events.onRawReviewCameraStatus((s) => alive && setStatus(s));
    return () => {
      alive = false;
      void un.then((f) => f()).catch(() => {});
    };
  }, []);
  return status;
}

export type CameraMonitor = {
  status: CameraStatus | undefined;
  diagnostics: CameraDiagnostics | undefined;
  checking: boolean;
  log: CameraLogLine[];
  /** Ripete la verifica completa. */
  recheck: () => Promise<CameraDiagnostics | undefined>;
};

/**
 * Stato vivo per il dialogo. La verifica completa gira su richiesta (e una volta all'apertura);
 * con `poll` attivo, ogni pochi secondi si rilegge solo lo stato leggero, mai durante una verifica.
 */
export function useCameraMonitor(poll: boolean): CameraMonitor {
  const [status, setStatus] = useState<CameraStatus>();
  const [diagnostics, setDiagnostics] = useState<CameraDiagnostics>();
  const [checking, setChecking] = useState(false);
  const [log, setLog] = useState<CameraLogLine[]>([]);
  const checkingRef = useRef(false);
  const stateRef = useRef<CameraStatus["state"] | undefined>(undefined);
  stateRef.current = status?.state;

  useEffect(() => {
    let alive = true;
    commands
      .rawReviewCameraLog()
      .then((l) => alive && setLog(l))
      .catch(() => {});
    commands
      .rawReviewCameraStatus()
      .then((s) => alive && setStatus((current) => current ?? s))
      .catch(() => {});
    const un = [
      events.onRawReviewCameraStatus((s) => alive && setStatus(s)),
      events.onRawReviewCameraDiagnostics((d) => {
        if (alive && checkingRef.current) setDiagnostics(d);
      }),
      events.onRawReviewCameraLog((l) => alive && setLog((lines) => appendLog(lines, l))),
    ];
    return () => {
      alive = false;
      un.forEach((p) => void p.then((f) => f()).catch(() => {}));
    };
  }, []);

  const recheck = useCallback(async () => {
    if (checkingRef.current) return undefined;
    checkingRef.current = true;
    setChecking(true);
    try {
      const result = await commands.rawReviewCameraDiagnostics();
      setDiagnostics(result);
      return result;
    } catch (e) {
      const detail = (e as { detail?: string } | null)?.detail ?? String(e);
      setLog((lines) => appendLog(lines, { at: Date.now(), level: "error", message: detail }));
      return undefined;
    } finally {
      checkingRef.current = false;
      setChecking(false);
      try {
        setStatus(await commands.rawReviewCameraStatus());
      } catch {
        // Resta l'ultimo stato noto.
      }
    }
  }, []);

  // Controllo leggero periodico, più rado se la fotocamera non risponde, fermo a finestra nascosta.
  useEffect(() => {
    if (!poll) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let cancelled = false;
    const tick = async () => {
      if (!document.hidden && !checkingRef.current) {
        try {
          const s = await commands.rawReviewCameraStatus();
          if (!cancelled) setStatus(s);
        } catch {
          // Si riprova al prossimo giro.
        }
      }
      if (!cancelled) timer = setTimeout(tick, nextPollDelay(stateRef.current, false) ?? 5000);
    };
    timer = setTimeout(tick, 5000);
    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [poll]);

  // La fotocamera è tornata pronta dopo un problema: si rifà la verifica una volta.
  const previous = useRef<CameraStatus["state"] | undefined>(undefined);
  useEffect(() => {
    const state = status?.state;
    if (
      state === "ready" &&
      previous.current &&
      previous.current !== "ready" &&
      diagnostics &&
      !diagnostics.ready
    )
      void recheck();
    previous.current = state;
  }, [status?.state, diagnostics, recheck]);

  return { status, diagnostics, checking, log, recheck };
}
