// Sessione di revisione delle Varianti RAW aperta nel modulo. Sta in uno store, non nello stato
// della pagina, così sopravvive allo smontaggio del modulo: tornando con Ctrl+9 (o dalla
// navigazione) si riparte dall'ultima sessione aperta, senza riaprire la cartella.
//
// Le scelte stanno nel backend (`ReviewState`); qui c'è la copia che l'interfaccia mostra, tenuta
// allineata con le risposte dei comandi. Il salvataggio su disco lo chiede `saveOpenReview`.
import { create } from "zustand";
import { commands } from "../lib/ipc";
import type { OpenedReview, PanelState, ShotDecision } from "../lib/ipc";
import {
  movePanel,
  panelCandidates,
  panelLock,
  showAllPanels,
  togglePanel,
} from "../modules/raw-review/project";
import { notifyError } from "./toasts";

interface RawReviewState {
  opened: OpenedReview | null;
  /** Posizione dello scatto mostrato nell'elenco `opened.shots`. */
  shotIndex: number;
  /** Rende corrente una sessione appena aperta, ripartendo dall'ultimo scatto visto. */
  setOpened: (opened: OpenedReview) => void;
  clear: () => void;
  setShotIndex: (index: number) => void;
  applyDecision: (decision: ShotDecision) => void;
  /** Cambia solo alcuni campi della decisione di uno scatto (aggiornamenti ottimistici). */
  patchDecision: (shotKey: string, patch: Partial<Omit<ShotDecision, "key">>) => void;
  applyPanels: (panels: PanelState) => void;
}

const EMPTY_DECISION = { rating: 0, winnerPreset: null, reviewed: false } as const;

export const useRawReview = create<RawReviewState>()((set) => ({
  opened: null,
  shotIndex: 0,
  setOpened: (opened) => {
    const last = opened.session.lastShotKey;
    const resumed = last ? opened.shots.findIndex((s) => s.key === last) : -1;
    set({ opened, shotIndex: Math.max(0, resumed) });
  },
  clear: () => set({ opened: null, shotIndex: 0 }),
  setShotIndex: (shotIndex) => set({ shotIndex }),
  applyDecision: (decision) =>
    set((s) =>
      s.opened
        ? {
            opened: {
              ...s.opened,
              session: {
                ...s.opened.session,
                decisions: { ...s.opened.session.decisions, [decision.key]: decision },
              },
            },
          }
        : s,
    ),
  patchDecision: (shotKey, patch) =>
    set((s) =>
      s.opened
        ? {
            opened: {
              ...s.opened,
              session: {
                ...s.opened.session,
                decisions: {
                  ...s.opened.session.decisions,
                  [shotKey]: {
                    ...EMPTY_DECISION,
                    ...s.opened.session.decisions?.[shotKey],
                    ...patch,
                    key: shotKey,
                  },
                },
              },
            },
          }
        : s,
    ),
  applyPanels: (panels) =>
    set((s) =>
      s.opened
        ? {
            opened: {
              ...s.opened,
              session: {
                ...s.opened.session,
                panelPresets: panels.panelPresets,
                hiddenPresets: panels.hiddenPresets,
              },
            },
          }
        : s,
    ),
}));

/** Pannelli visibili e nascosti della sessione aperta. */
export function currentPanels(opened: OpenedReview): PanelState {
  return {
    panelPresets: opened.session.panelPresets,
    hiddenPresets: opened.session.hiddenPresets ?? [],
  };
}

/** Preset che si possono mostrare come pannelli (guida, altri preset e anteprima RAW). */
export function candidatesOf(opened: OpenedReview): string[] {
  return panelCandidates(opened.layout, opened.presets, opened.guidePreset);
}

/** Attesa dopo l'ultima modifica prima del salvataggio automatico. */
export const AUTOSAVE_DELAY_MS = 500;

// Le modifiche partono verso il backend una alla volta, nell'ordine in cui l'utente le fa: una
// risposta in ritardo non può sovrascrivere una modifica più recente, e il salvataggio su disco
// parte sempre dopo l'ultima modifica.
let tail: Promise<unknown> = Promise.resolve();
let inFlight = 0;
function track<T>(run: Promise<T>): Promise<T> {
  inFlight += 1;
  const done = run.catch(() => undefined).then(() => {
    inFlight -= 1;
  });
  tail = Promise.all([tail, done]);
  return run;
}
function serial<T>(task: () => Promise<T>): Promise<T> {
  return track(tail.then(task, task));
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;
function cancelAutosave() {
  if (saveTimer !== null) clearTimeout(saveTimer);
  saveTimer = null;
}

/** Chiede il salvataggio automatico dopo una pausa nelle modifiche (scelta, stelle, pannelli, scatto). */
export function scheduleSave(): void {
  cancelAutosave();
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void saveOpenReview();
  }, AUTOSAVE_DELAY_MS);
}

/**
 * Salva subito la sessione aperta nei dati dell'app, dopo le modifiche ancora in volo: da
 * chiamare prima di lasciare il modulo. Annulla il salvataggio automatico in attesa.
 */
export async function saveOpenReview(): Promise<void> {
  cancelAutosave();
  // Senza modifiche in volo si salva subito (anche allo smontaggio); altrimenti si aspetta.
  if (inFlight > 0) await tail;
  const id = useRawReview.getState().opened?.sessionId;
  if (!id) return;
  try {
    await commands.rawReviewSave(id);
  } catch {
    // La sessione è stata chiusa nel frattempo: l'ultimo salvataggio resta valido.
  }
}

// Chiusura dell'app o ricarica della finestra: salvataggio al meglio delle possibilità.
if (typeof window !== "undefined") {
  window.addEventListener("pagehide", () => void saveOpenReview());
}

/** Passa a un altro scatto e lo ricorda nella sessione. */
export function selectShot(index: number): void {
  const { opened, shotIndex, setShotIndex } = useRawReview.getState();
  if (!opened?.shots.length) return;
  const next = Math.min(opened.shots.length - 1, Math.max(0, index));
  if (next === shotIndex) return;
  setShotIndex(next);
  const { sessionId } = opened;
  const key = opened.shots[next].key;
  void track(Promise.resolve(commands.rawReviewSetLastShot(sessionId, key))).catch(() => {});
  scheduleSave();
}

/**
 * Sceglie la variante migliore dello scatto indicato (e lo segna come rivisto). La scelta si
 * vede subito; se il backend la rifiuta torna quella di prima. Non cambia lo scatto mostrato.
 */
export function chooseWinner(shotKey: string, preset: string): Promise<void> {
  const { opened, patchDecision } = useRawReview.getState();
  if (!opened) return Promise.resolve();
  const { sessionId } = opened;
  const before = opened.session.decisions?.[shotKey];
  patchDecision(shotKey, { winnerPreset: preset, reviewed: true });
  scheduleSave();
  return serial(async () => {
    try {
      let decision = await commands.rawReviewSetWinner(sessionId, shotKey, preset);
      if (!decision.reviewed) {
        decision = await commands.rawReviewSetReviewed(sessionId, shotKey, true);
      }
      // Solo scelta e revisione: la valutazione può essere cambiata nel frattempo.
      useRawReview.getState().patchDecision(shotKey, {
        winnerPreset: decision.winnerPreset,
        reviewed: decision.reviewed,
      });
    } catch (e) {
      useRawReview.getState().patchDecision(shotKey, {
        winnerPreset: before?.winnerPreset ?? null,
        reviewed: before?.reviewed ?? false,
      });
      notifyError(e);
    }
  });
}

/** Assegna da 0 a 5 stelle allo scatto indicato, con aggiornamento subito visibile. */
export function setRating(shotKey: string, rating: number): Promise<void> {
  const { opened, patchDecision } = useRawReview.getState();
  if (!opened) return Promise.resolve();
  const { sessionId } = opened;
  const value = Math.min(5, Math.max(0, Math.round(rating)));
  const before = opened.session.decisions?.[shotKey];
  // Valutare segna lo scatto come rivisto (come in FujiView).
  patchDecision(shotKey, { rating: value, reviewed: true });
  scheduleSave();
  return serial(async () => {
    try {
      const decision = await commands.rawReviewSetRating(sessionId, shotKey, value);
      if (!decision?.reviewed) await commands.rawReviewSetReviewed(sessionId, shotKey, true);
    } catch (e) {
      patchDecision(shotKey, {
        rating: before?.rating ?? 0,
        reviewed: before?.reviewed ?? false,
      });
      notifyError(e);
    }
  });
}

type PanelEdit = (state: PanelState) => PanelState;

/** Applica subito una modifica dei pannelli e la conferma con il backend; in errore la annulla. */
async function editPanels(
  edit: PanelEdit,
  send: (sessionId: string) => Promise<PanelState>,
): Promise<void> {
  const { opened, applyPanels } = useRawReview.getState();
  if (!opened) return;
  const before = currentPanels(opened);
  const next = edit(before);
  if (next === before) return;
  applyPanels(next);
  scheduleSave();
  await serial(async () => {
    try {
      applyPanels(await send(opened.sessionId));
    } catch (e) {
      applyPanels(before);
      notifyError(e);
    }
  });
}

export function toggleVisible(preset: string): Promise<void> {
  const opened = useRawReview.getState().opened;
  if (!opened || panelLock(opened.session.panelPresets, preset)) return Promise.resolve();
  const candidates = candidatesOf(opened);
  return editPanels(
    (s) => togglePanel(s, preset, candidates),
    (id) => commands.rawReviewTogglePreset(id, preset),
  );
}

export function movePreset(preset: string, to: number): Promise<void> {
  return editPanels(
    (s) => movePanel(s, preset, to),
    (id) => commands.rawReviewMovePreset(id, preset, to),
  );
}

export function showAllPresets(): Promise<void> {
  const opened = useRawReview.getState().opened;
  if (!opened) return Promise.resolve();
  const candidates = candidatesOf(opened);
  return editPanels(
    (s) => showAllPanels(s, candidates),
    (id) => commands.rawReviewShowAllPresets(id),
  );
}
