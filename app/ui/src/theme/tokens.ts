// Design token di photo-cat, ricavati dai mockup (canvas "photo-cat UI mockups").
// Unica fonte di verità: `applyTheme` li scrive come CSS custom properties su :root.

export type ThemeName = "dark" | "light";

export interface ThemeTokens {
  bg: string;
  panel: string;
  panelAlt: string;
  toolbar: string;
  raised: string;
  border: string;
  text: string;
  textSecondary: string;
  textSoft: string;
  textPlaceholder: string;
  accent: string;
  accentText: string;
  onAccent: string;
  accentSoftBg: string;
  accentSoftText: string;
  selectionBg: string;
  selectionBorder: string;
  treeSelected: string;
  ok: string;
  warn: string;
  danger: string;
  dangerBorder: string;
  thumbBg: string;
  okBg: string;
  warnBg: string;
  dangerBg: string;
  logBg: string;
}

export const themes: Record<ThemeName, ThemeTokens> = {
  dark: {
    bg: "#1B1B1C",
    panel: "#232325",
    panelAlt: "#1F1F21",
    toolbar: "#202022",
    raised: "#2C2C2F",
    border: "#3A3A3E",
    text: "#E8E8EA",
    textSecondary: "#A9A9AE",
    textSoft: "#C9C9CE",
    textPlaceholder: "#8E8E95",
    accent: "#3B8EEA",
    accentText: "#6AAEF5",
    onAccent: "#0B1726",
    accentSoftBg: "#233449",
    accentSoftText: "#C9DCF2",
    selectionBg: "#1E3047",
    selectionBorder: "#2F4D73",
    treeSelected: "#2A3A50",
    ok: "#4FC3B0",
    warn: "#E0B252",
    danger: "#F2A38F",
    dangerBorder: "#6B3A30",
    thumbBg: "#34393A",
    okBg: "#1E3A36",
    warnBg: "#3A3320",
    dangerBg: "#3D2420",
    logBg: "#141415",
  },
  light: {
    bg: "#F4F4F5",
    panel: "#FFFFFF",
    panelAlt: "#FAFAFA",
    toolbar: "#F7F7F8",
    raised: "#ECECEE",
    border: "#D4D4D8",
    text: "#1B1B1C",
    textSecondary: "#5A5A60",
    textSoft: "#3F3F45",
    textPlaceholder: "#6E6E75",
    accent: "#1F6FD1",
    accentText: "#1A5FB4",
    onAccent: "#FFFFFF",
    accentSoftBg: "#DCE9F9",
    accentSoftText: "#163E6E",
    selectionBg: "#E3EEFB",
    selectionBorder: "#A9C8EE",
    treeSelected: "#D6E6F8",
    ok: "#176A5D",
    warn: "#8A5A00",
    danger: "#B3361C",
    dangerBorder: "#E3A79A",
    thumbBg: "#DADDE0",
    okBg: "#DDF1EC",
    warnBg: "#F7ECD3",
    dangerBg: "#F9E1DB",
    logBg: "#FBFBFC",
  },
};

/** Coppie testo/sfondo che devono rispettare il contrasto minimo 4,5:1. */
export const contrastPairs: Array<[keyof ThemeTokens, keyof ThemeTokens]> = [
  ["text", "bg"],
  ["text", "panel"],
  ["text", "raised"],
  ["textSecondary", "bg"],
  ["textSecondary", "panel"],
  ["textSecondary", "panelAlt"],
  ["textSecondary", "toolbar"],
  ["textSoft", "panel"],
  ["accentText", "bg"],
  ["accentText", "panel"],
  ["onAccent", "accent"],
  ["accentSoftText", "accentSoftBg"],
  ["text", "selectionBg"],
  ["ok", "panel"],
  ["warn", "panel"],
  ["danger", "panel"],
  ["danger", "bg"],
  ["ok", "okBg"],
  ["warn", "warnBg"],
  ["danger", "dangerBg"],
  ["textSoft", "logBg"],
];

/** Palette fissa dei tag: chiavi come `pc_store::app_store::tags::PALETTE`. */
export const TAG_COLORS = ["blue", "teal", "yellow", "coral", "purple", "gray"] as const;
export type TagColor = (typeof TAG_COLORS)[number];

export const tagPalette: Record<ThemeName, Record<TagColor, string>> = {
  dark: {
    blue: "#3B8EEA",
    teal: "#4FC3B0",
    yellow: "#E0B252",
    coral: "#F2A38F",
    purple: "#B59CF0",
    gray: "#A9A9AE",
  },
  light: {
    blue: "#1F6FD1",
    teal: "#176A5D",
    yellow: "#8A5A00",
    coral: "#B3361C",
    purple: "#6E4BC2",
    gray: "#5A5A60",
  },
};

/** Colori della striscia del dispositivo nella Sequenza (per indice del gruppo di sorgente). */
export const DEVICE_COLORS = 8;
const devicePalette: Record<ThemeName, string[]> = {
  dark: ["#3B8EEA", "#E0B252", "#4FC3B0", "#F2A38F", "#B59CF0", "#7FD36B", "#F07FB4", "#A9A9AE"],
  light: ["#1F6FD1", "#8A5A00", "#176A5D", "#B3361C", "#6E4BC2", "#3C7A25", "#B02A6E", "#5A5A60"],
};

/** Variabile CSS del colore di un tag (chiave sconosciuta: grigio). */
export const tagColorVar = (key: string) =>
  `var(--pc-tag-${(TAG_COLORS as readonly string[]).includes(key) ? key : "gray"})`;

export const deviceColorVar = (index: number) =>
  `var(--pc-device-${((index % DEVICE_COLORS) + DEVICE_COLORS) % DEVICE_COLORS})`;

export const fonts = {
  ui: "'Source Sans 3', system-ui, sans-serif",
  mono: "'JetBrains Mono', ui-monospace, monospace",
};

const cssName = (key: string) => `--pc-${key.replace(/[A-Z]/g, (m) => `-${m.toLowerCase()}`)}`;

export function applyTheme(name: ThemeName, root: HTMLElement = document.documentElement) {
  const t = themes[name];
  for (const [k, v] of Object.entries(t)) root.style.setProperty(cssName(k), v);
  for (const [k, v] of Object.entries(tagPalette[name])) root.style.setProperty(`--pc-tag-${k}`, v);
  devicePalette[name].forEach((v, i) => root.style.setProperty(`--pc-device-${i}`, v));
  root.style.setProperty("--pc-font-ui", fonts.ui);
  root.style.setProperty("--pc-font-mono", fonts.mono);
  root.dataset.theme = name;
  root.style.colorScheme = name;
}
