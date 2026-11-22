// Logica pura dei pannelli di confronto (porting di FujiView `project.ts`). Lo stato vero sta
// nel backend (`Session`): queste funzioni ne ripetono le regole per decidere cosa mostrare
// e per aggiornare l'interfaccia subito, prima della risposta dei comandi dei pannelli.
import type { PanelState, ReviewLayoutKind } from "../../lib/ipc";

/** Un pannello per preset, al massimo quanti i tasti Maiusc+1–9. */
export const MAX_PANELS = 9;

/** Preset della pseudo-variante estratta dal RAF ("Anteprima RAW"): si valuta, non si sceglie. */
export const RAW_PREVIEW = "raw-preview";

export const isRawPreview = (preset: string): boolean => preset === RAW_PREVIEW;

/** Griglia per n pannelli: una riga fino a 3, poi la forma più quadrata (4 → 2×2, 5–6 → 3×2, 7–9 → 3×3). */
export function panelGrid(count: number): { columns: number; rows: number } {
  const n = Math.max(1, count);
  const columns = n <= 3 ? n : Math.ceil(Math.sqrt(n));
  return { columns, rows: Math.ceil(n / columns) };
}

/**
 * Pannelli disponibili nell'ordine predefinito: la guida, gli altri preset e, nelle cartelle con
 * i RAF, l'"Anteprima RAW" in fondo (come `panel_candidates` del backend).
 */
export function panelCandidates(
  layout: ReviewLayoutKind,
  presets: readonly string[],
  guidePreset: string | null | undefined,
): string[] {
  const out = guidePreset ? [guidePreset] : [];
  out.push(...presets.filter((p) => p !== guidePreset));
  if (layout !== "presets") out.push(RAW_PREVIEW);
  return out;
}

/** Perché un pannello non si può mostrare o nascondere (`null` se si può). */
export type PanelLock = "last" | "max";

export function panelLock(panels: readonly string[], preset: string): PanelLock | null {
  if (panels.includes(preset)) return panels.length <= 1 ? "last" : null;
  return panels.length >= MAX_PANELS ? "max" : null;
}

/** Mostra o nasconde il pannello di un preset. L'ultimo pannello visibile resta, al massimo MAX_PANELS. */
export function togglePanel(
  state: PanelState,
  preset: string,
  candidates: readonly string[],
): PanelState {
  const shown = state.panelPresets.includes(preset);
  if (shown) {
    if (panelLock(state.panelPresets, preset)) return state;
    return {
      panelPresets: state.panelPresets.filter((p) => p !== preset),
      hiddenPresets: [...state.hiddenPresets.filter((p) => p !== preset), preset],
    };
  }
  if (panelLock(state.panelPresets, preset) || !candidates.includes(preset)) return state;
  return {
    panelPresets: [...state.panelPresets, preset],
    hiddenPresets: state.hiddenPresets.filter((p) => p !== preset),
  };
}

/** Mostra di nuovo tutti i preset (fino a MAX_PANELS), con i pannelli attuali per primi. */
export function showAllPanels(state: PanelState, candidates: readonly string[]): PanelState {
  const panelPresets = [
    ...state.panelPresets,
    ...candidates.filter((p) => !state.panelPresets.includes(p)),
  ].slice(0, MAX_PANELS);
  return { panelPresets, hiddenPresets: [] };
}

/** Sposta un pannello visibile alla posizione `to`; senza effetto se non è valida. */
export function movePanel(state: PanelState, preset: string, to: number): PanelState {
  const from = state.panelPresets.indexOf(preset);
  if (from < 0 || to < 0 || to >= state.panelPresets.length || from === to) return state;
  const panelPresets = state.panelPresets.filter((p) => p !== preset);
  panelPresets.splice(to, 0, preset);
  return { ...state, panelPresets };
}
