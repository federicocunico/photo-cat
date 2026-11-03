// Copia delle preferenze dell'interfaccia nella cartella dell'app
// (`ui-prefs.json`, comandi uiPrefsLoad/uiPrefsSave). Il profilo WebView2 con
// localStorage può essere cancellato dal disinstallatore; all'avvio le chiavi
// mancanti vengono ripristinate dal file prima che i negozi si idratino.
import type { StateStorage } from "zustand/middleware";
import { commands, isTauri } from "./ipc";

/** Chiavi dei negozi persistiti (`persist({ name })`). */
export const UI_PREF_KEYS = [
  "photo-cat.layout",
  "photo-cat.library",
  "photo-cat.floating",
  "photo-cat.palette",
  "photo-cat.guide",
];

const SAVE_DELAY = 500;
let timer: ReturnType<typeof setTimeout> | null = null;

function current(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const k of UI_PREF_KEYS) {
    const v = localStorage.getItem(k);
    if (v != null) out[k] = v;
  }
  return out;
}

/** Salva subito le preferenze correnti nel file. */
export async function flushUiPrefs() {
  if (timer) clearTimeout(timer);
  timer = null;
  if (!isTauri()) return;
  try {
    await commands.uiPrefsSave(current());
  } catch {
    // Le preferenze restano in localStorage; si riprova alla prossima modifica.
  }
}

function scheduleSave() {
  if (!isTauri()) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void flushUiPrefs(), SAVE_DELAY);
}

/**
 * Da chiamare prima di importare l'app: copia in localStorage le preferenze
 * del file che mancano (profilo WebView2 nuovo o cancellato), poi salva nel
 * file lo stato corrente (prima esecuzione dopo un aggiornamento).
 */
export async function restoreUiPrefs() {
  if (!isTauri()) return;
  try {
    const saved = await commands.uiPrefsLoad();
    for (const k of UI_PREF_KEYS) {
      const v = saved[k];
      if (localStorage.getItem(k) == null && v != null) localStorage.setItem(k, v);
    }
  } catch {
    return;
  }
  await flushUiPrefs();
}

/** Storage per `persist`: localStorage più copia ritardata nel file. */
export const mirroredStorage: StateStorage = {
  getItem: (k) => localStorage.getItem(k),
  setItem: (k, v) => {
    localStorage.setItem(k, v);
    scheduleSave();
  },
  removeItem: (k) => {
    localStorage.removeItem(k);
    scheduleSave();
  },
};

if (typeof window !== "undefined") {
  window.addEventListener("pagehide", () => {
    if (timer) void flushUiPrefs();
  });
}
