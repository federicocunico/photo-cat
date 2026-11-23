// Dati della barra dei menu, separati dal componente (application-menu).
import { shortcutText, type Command, type MenuId } from "../../commands/registry";

export const MENUS: MenuId[] = ["file", "edit", "view", "go", "tools", "help"];

export const isMacPlatform = () => /Mac|iPhone|iPad/.test(globalThis.navigator?.userAgent ?? "");

/** "V&ai" → testo "Vai" con la lettera d'accesso "a" in posizione 1. */
export function accessKey(label: string): { text: string; key: string; index: number } {
  const at = label.indexOf("&");
  const text = label.replace("&", "");
  return at < 0
    ? { text, key: text[0]!.toLowerCase(), index: 0 }
    : { text, key: text[at]!.toLowerCase(), index: at };
}

/** Testo dei tasti di un comando, identico a quello della palette. */
export function keysOf(command: Command): string | undefined {
  return command.keys ?? (command.shortcut ? shortcutText(command.shortcut) : undefined);
}
