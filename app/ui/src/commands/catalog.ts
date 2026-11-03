// Catalogo delle scorciatoie per il foglio "?": voci del registro solo da mostrare
// (senza esecuzione), anche per i moduli non aperti. catalog.test.ts verifica che ogni
// scorciatoia registrata da un comando vero sia qui, così il foglio non diverge dai tasti.
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { useCommands, type Command, type CommandGroup } from "./registry";

type Entry = { id: string; group: CommandGroup; keys: string; label: string };

export function entries(t: TFunction): Entry[] {
  const k = (id: string) => t(`shortcuts.items.${id}`);
  const e = (group: CommandGroup, id: string, keys: string): Entry => ({
    id: `doc.${group}.${id}`,
    group,
    keys,
    label: k(id),
  });
  return [
    e("general", "palette", "Ctrl+K"),
    e("general", "undo", "Ctrl+Z"),
    e("general", "sheet", "?"),
    e("navigation", "modules", "Ctrl+1…7"),
    e("navigation", "settings", "Ctrl+,"),
    e("navigation", "nav", "Ctrl+B"),
    e("library", "inspector", "Ctrl+I"),
    e("library", "panels", "Ctrl+Maiusc+B"),
    e("library", "move", "← → ↑ ↓"),
    e("library", "page", "Pag↑ Pag↓ Home Fine"),
    e("library", "extend", "Maiusc+frecce"),
    e("library", "toggle", "Ctrl+Spazio"),
    e("library", "all", "Ctrl+A"),
    e("library", "open", "Spazio, Invio"),
    e("library", "deselect", "Esc"),
    e("library", "tags", "1–9"),
    e("library", "untag", "0"),
    e("library", "exclude", "Canc"),
    e("library", "menu", "Maiusc+F10"),
    e("viewer", "browse", "← →"),
    e("viewer", "tags", "1–9, 0"),
    e("viewer", "info", "I"),
    e("viewer", "close", "Esc, Spazio"),
    e("sequence", "session", "A"),
    e("sequence", "open", "Spazio"),
    e("sequence", "tags", "1–9, 0"),
    e("people", "groups", "↑ ↓"),
    e("people", "face", "Spazio"),
    e("people", "whoIs", "N"),
    e("people", "moveTo", "M"),
    e("people", "notSame", "X"),
    e("people", "merge", "U"),
    e("people", "ignore", "I"),
    e("people", "undo", "Ctrl+Z"),
  ];
}

/** Registra il catalogo per tutta la durata dell'app. */
export function useShortcutCatalog() {
  const { t } = useTranslation();
  useCommands(
    entries(t).map((x): Command => ({
      id: x.id,
      label: x.label,
      group: x.group,
      keys: x.keys,
      palette: false,
    })),
  );
}

/** Ordine delle aree nel foglio. */
export const SHEET_GROUPS: CommandGroup[] = [
  "general",
  "navigation",
  "library",
  "viewer",
  "sequence",
  "people",
];
