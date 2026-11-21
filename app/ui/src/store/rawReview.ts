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
  applyPanels: (panels: PanelState) => void;
}

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

/** Salva la sessione aperta nei dati dell'app: da chiamare prima di lasciare il modulo. */
export async function saveOpenReview(): Promise<void> {
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
  void commands.rawReviewSetLastShot(opened.sessionId, opened.shots[next].key).catch(() => {});
}

/** Sceglie la variante migliore dello scatto indicato (e lo segna come rivisto). */
export async function chooseWinner(shotKey: string, preset: string): Promise<void> {
  const { opened, applyDecision } = useRawReview.getState();
  if (!opened) return;
  try {
    let decision = await commands.rawReviewSetWinner(opened.sessionId, shotKey, preset);
    if (!decision.reviewed) {
      decision = await commands.rawReviewSetReviewed(opened.sessionId, shotKey, true);
    }
    applyDecision(decision);
  } catch (e) {
    notifyError(e);
  }
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
  try {
    applyPanels(await send(opened.sessionId));
  } catch (e) {
    applyPanels(before);
    notifyError(e);
  }
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
