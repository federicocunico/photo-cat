// Stato della vista Sequenza: ambito, selezione e vista interna (sequenza o export).
import { create } from "zustand";
import type { SequenceScope } from "../lib/ipc";

interface SequenceState {
  scope: SequenceScope;
  selected: number[];
  /** Ultimo elemento cliccato (ancora per Maiusc+clic, sessione per il tasto A). */
  focused: number | null;
  view: "sequence" | "export";
  /** Conteggi per la barra di stato. */
  stats: { total: number; devices: number; sessions: number };
  setScope: (s: SequenceScope) => void;
  setSelection: (ids: number[], focused?: number | null) => void;
  setView: (v: "sequence" | "export") => void;
  setStats: (s: SequenceState["stats"]) => void;
}

export const useSequence = create<SequenceState>()((set) => ({
  scope: { scope: "library" },
  selected: [],
  focused: null,
  view: "sequence",
  stats: { total: 0, devices: 0, sessions: 0 },
  setScope: (scope) => set({ scope, selected: [], focused: null }),
  setSelection: (selected, focused) =>
    set((s) => ({ selected, focused: focused === undefined ? s.focused : focused })),
  setView: (view) => set({ view }),
  setStats: (stats) => set({ stats }),
}));

/** `±H:MM:SS`, secondi semplici o `+2 min 10 s` → secondi; `null` se non valido. */
export function parseShift(text: string): number | null {
  const s = text.trim().replace(",", ".");
  if (!s) return 0;
  const sign = s.startsWith("-") ? -1 : 1;
  const body = s.replace(/^[+-]/, "").trim();
  if (/^\d+$/.test(body)) return sign * Number(body);
  const hms = body.match(/^(\d+):(\d{1,2})(?::(\d{1,2}))?$/);
  if (hms) {
    const [h, m, sec] = [Number(hms[1]), Number(hms[2]), Number(hms[3] ?? 0)];
    if (m > 59 || sec > 59) return null;
    return sign * (h * 3600 + m * 60 + sec);
  }
  const parts = [...body.matchAll(/(\d+)\s*(h|min|m|s)\b/g)];
  if (!parts.length || parts.map((p) => p[0]).join("").length < body.replace(/\s/g, "").length)
    return null;
  let total = 0;
  for (const [, n, unit] of parts)
    total += Number(n) * (unit === "h" ? 3600 : unit === "s" ? 1 : 60);
  return sign * total;
}

/** Secondi → `+2 min 10 s` (come `pc_store::timeline::shift_text`). */
export function shiftText(s: number): string {
  const sign = s < 0 ? "-" : "+";
  const a = Math.abs(s);
  const [h, m, sec] = [Math.floor(a / 3600), Math.floor((a % 3600) / 60), a % 60];
  const parts: string[] = [];
  if (h) parts.push(`${h} h`);
  if (m) parts.push(`${m} min`);
  if (sec || !parts.length) parts.push(`${sec} s`);
  return `${sign}${parts.join(" ")}`;
}

/** Minuti → `+02:00`. */
export function offsetText(min: number): string {
  const sign = min < 0 ? "-" : "+";
  const a = Math.abs(min);
  return `${sign}${String(Math.floor(a / 60)).padStart(2, "0")}:${String(a % 60).padStart(2, "0")}`;
}

/** `AAAA-MM-GG HH:MM:SS` → `HH:MM:SS`. */
export const timeOf = (local: string) => local.slice(11, 19);
