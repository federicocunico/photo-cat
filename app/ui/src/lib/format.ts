// Formattazione di numeri, durate e dimensioni secondo la lingua corrente.

import i18n from "../i18n";

const locale = () => (i18n.language === "en" ? "en-GB" : "it-IT");

export const fmtInt = (n: number) => new Intl.NumberFormat(locale()).format(n);

export const fmtDec = (n: number, digits = 1) =>
  new Intl.NumberFormat(locale(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(n);

/** 3725 → "1 h 02 min", 95 → "1 min 35 s", 12 → "12 s" */
export function fmtDuration(secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  if (h > 0) return `${h} h ${String(m).padStart(2, "0")} min`;
  if (m > 0) return `${m} min ${String(r).padStart(2, "0")} s`;
  return `${r} s`;
}

/** Tempo residuo: precisione proporzionata alla durata, così l'etichetta non cambia a ogni secondo. */
export function fmtEta(secs: number): string {
  const s = Math.max(0, secs);
  if (s < 60) return fmtDuration(Math.max(5, Math.ceil(s / 5) * 5));
  if (s < 600) return fmtDuration(Math.ceil(s / 10) * 10);
  const m = Math.ceil(s / 60);
  return m >= 60 ? `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")} min` : `${m} min`;
}

export const mbToGb = (mb: number) => fmtDec(mb / 1024, 1);

export function fmtBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${i === 0 ? fmtInt(v) : fmtDec(v, 1)} ${units[i]}`;
}

/** Data e ora nella lingua dell'app (non in quella del sistema). */
export const fmtDateTime = (d: string | number | Date) =>
  new Intl.DateTimeFormat(locale(), { dateStyle: "short", timeStyle: "medium" }).format(
    new Date(d),
  );

export const fmtTime = (d: string | number | Date) =>
  new Intl.DateTimeFormat(locale(), { timeStyle: "medium" }).format(new Date(d));

export const fmtDate = (d: string | number | Date) =>
  new Intl.DateTimeFormat(locale(), { dateStyle: "medium" }).format(new Date(d));

/**
 * Anno, mese o giorno del filtro per data (`AAAA`, `AAAA-MM`, `AAAA-MM-GG`)
 * nella lingua corrente. `short`: solo il nome del mese ("maggio") o il giorno
 * con la settimana ("dom 14"); altrimenti per intero ("14 maggio 2023").
 */
export function fmtPeriod(p: string, short = false): string {
  const [y, m, d] = p.split("-").map(Number);
  if (!m) return p;
  // In UTC, così la data non si sposta con il fuso del computer.
  const date = new Date(Date.UTC(y, m - 1, d || 1));
  const opts: Intl.DateTimeFormatOptions = short
    ? d
      ? { weekday: "short", day: "numeric" }
      : { month: "long" }
    : d
      ? { day: "numeric", month: "long", year: "numeric" }
      : { month: "long", year: "numeric" };
  return new Intl.DateTimeFormat(locale(), { ...opts, timeZone: "UTC" }).format(date);
}
