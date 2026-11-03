// Registro dei comandi (design D7): navigazione, menu, palette e foglio delle scorciatoie
// leggono tutti da qui. Un solo gestore globale della tastiera esegue le scorciatoie.
import { useEffect, useRef } from "react";
import { create } from "zustand";
import { isTyping, modalOpen } from "../lib/keys";

export type CommandGroup =
  "general" | "navigation" | "library" | "selection" | "viewer" | "sequence" | "people";

/** Combinazione di tasti; `mod` è Ctrl (⌘ su macOS). */
export type Shortcut = {
  key: string;
  mod?: boolean;
  shift?: boolean;
  alt?: boolean;
};

export type Command = {
  id: string;
  /** Etichetta in parole (già tradotta). */
  label: string;
  group: CommandGroup;
  /** Scorciatoia gestita dal gestore globale. */
  shortcut?: Shortcut;
  /**
   * Tasti solo da mostrare (scorciatoie gestite altrove, es. dentro la griglia), per il
   * foglio delle scorciatoie e i menu.
   */
  keys?: string;
  /** Il comando è disponibile ora (es. "c'è una selezione"). */
  when?: () => boolean;
  /** Attiva anche nei campi di testo (solo Ctrl+K ed Esc, per la spec). */
  inInputs?: boolean;
  /** Mostrato nella palette (predefinito: sì, se ha `run`). */
  palette?: boolean;
  /** Parole in più per la ricerca nella palette. */
  keywords?: string;
  run?: () => unknown;
};

type State = { commands: Command[] };

/** Comandi registrati, in ordine di registrazione (l'ultimo vince sui tasti uguali). */
export const useCommandStore = create<State>(() => ({ commands: [] }));

const isMac = () => /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "");

/** Testo della scorciatoia nella lingua dei tasti del sistema, es. "Ctrl+Maiusc+B". */
export function shortcutText(s: Shortcut, shiftName = "Maiusc"): string {
  const parts: string[] = [];
  if (s.mod) parts.push(isMac() ? "⌘" : "Ctrl");
  if (s.alt) parts.push(isMac() ? "⌥" : "Alt");
  if (s.shift) parts.push(shiftName);
  parts.push(s.key.length === 1 ? s.key.toUpperCase() : s.key);
  return parts.join("+");
}

function matches(s: Shortcut, e: KeyboardEvent): boolean {
  const mod = e.ctrlKey || e.metaKey;
  if (Boolean(s.mod) !== mod || Boolean(s.alt) !== e.altKey) return false;
  // "?" si ottiene con Maiusc: per i simboli conta il carattere, non il tasto Maiusc.
  const symbol = s.key.length === 1 && !/[a-z0-9]/i.test(s.key);
  if (!symbol && Boolean(s.shift) !== e.shiftKey) return false;
  return e.key.toLowerCase() === s.key.toLowerCase();
}

function onKeyDown(e: KeyboardEvent) {
  if (e.defaultPrevented) return;
  const typing = isTyping(e.target);
  const modal = modalOpen();
  const list = useCommandStore.getState().commands;
  for (let i = list.length - 1; i >= 0; i--) {
    const c = list[i];
    if (!c.shortcut || !c.run || !matches(c.shortcut, e)) continue;
    if ((typing || modal) && !c.inInputs) continue;
    if (c.when && !c.when()) continue;
    e.preventDefault();
    void c.run();
    return;
  }
}

let listening = false;
function sync() {
  const any = useCommandStore.getState().commands.some((c) => c.shortcut);
  if (any && !listening) window.addEventListener("keydown", onKeyDown);
  if (!any && listening) window.removeEventListener("keydown", onKeyDown);
  listening = any;
}

/** Registra comandi; restituisce la funzione che li toglie. */
export function registerCommands(commands: Command[]): () => void {
  useCommandStore.setState((s) => ({ commands: [...s.commands, ...commands] }));
  sync();
  return () => {
    useCommandStore.setState((s) => ({
      commands: s.commands.filter((c) => !commands.includes(c)),
    }));
    sync();
  };
}

/**
 * Registra i comandi finché il componente è montato. `run` e `when` leggono sempre
 * l'ultima versione passata, così il registro non cambia a ogni render.
 */
export function useCommands(commands: Command[], enabled = true) {
  const latest = useRef(commands);
  latest.current = commands;
  const signature = commands
    .map((c) => `${c.id}|${c.label}|${c.keys ?? ""}|${JSON.stringify(c.shortcut ?? null)}`)
    .join(";");
  useEffect(() => {
    if (!enabled) return;
    const stable = latest.current.map((c, i): Command => ({
      ...c,
      when: c.when && (() => latest.current[i]?.when?.() ?? false),
      run: c.run && (() => latest.current[i]?.run?.()),
    }));
    return registerCommands(stable);
  }, [signature, enabled]);
}

/** Comandi disponibili adesso, per la palette (l'ultimo con lo stesso id vince). */
export function availableCommands(): Command[] {
  const seen = new Map<string, Command>();
  for (const c of useCommandStore.getState().commands) seen.set(c.id, c);
  return [...seen.values()].filter((c) => c.palette !== false && c.run && (!c.when || c.when()));
}
