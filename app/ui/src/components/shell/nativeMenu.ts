// Menu nativo di macOS (application-menu, "Menu su macOS"): le stesse voci della barra dei menu,
// prese dal registro, vanno al backend con `set_app_menu`; un clic torna come `menu://command`
// e il registro esegue il comando con quell'id. Su Windows e Linux la barra è in `MenuBar`.
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { commands, events, isTauri, type MenuItemSpec, type MenuLabels } from "../../lib/ipc";
import {
  commandEnabled,
  latestCommands,
  useCommandStore,
  type Command,
  type Shortcut,
} from "../../commands/registry";
import { MENUS, accessKey } from "./menuBarModel";

/** Scorciatoia nel formato degli acceleratori di Tauri ("CmdOrCtrl+Shift+I"). */
export function toAccelerator(s: Shortcut): string {
  const parts: string[] = [];
  if (s.mod) parts.push("CmdOrCtrl");
  if (s.alt) parts.push("Alt");
  if (s.shift) parts.push("Shift");
  parts.push(s.key.length === 1 ? s.key.toUpperCase() : s.key);
  return parts.join("+");
}

/** Voci del menu nativo dal registro (i segnaposto restano, disattivati). */
export function nativeMenuItems(list: Command[] = latestCommands()): MenuItemSpec[] {
  return list.flatMap((c) =>
    (c.menu ? [c.menu].flat() : []).map((p) => ({
      id: c.id,
      label: c.label,
      menu: p.menu,
      section: p.section,
      order: p.order,
      submenu: p.submenu ?? null,
      accelerator: c.shortcut ? toAccelerator(c.shortcut) : null,
      enabled: commandEnabled(c),
      checked: c.checked ? c.checked() : null,
    })),
  );
}

export function nativeMenuLabels(t: TFunction): MenuLabels {
  return {
    appName: "photo-cat",
    about: t("menu.about"),
    settings: `${t("modules.settings")}…`,
    quit: t("menu.quitApp"),
    menus: MENUS.map((id) => [id, accessKey(t(`menu.names.${id}`)).text]),
  };
}

/** Esegue il comando scelto nel menu nativo, se ora si può usare. */
export function runMenuCommand(id: string): boolean {
  const command = latestCommands().find((c) => c.id === id);
  if (!command || !commandEnabled(command)) return false;
  void command.run?.();
  return true;
}

export function useNativeMenu(enabled: boolean) {
  const { t, i18n } = useTranslation();
  const registry = useCommandStore((s) => s.commands);
  useEffect(() => {
    if (!enabled || !isTauri()) return;
    // Raggruppa le registrazioni dei moduli che montano insieme in un solo aggiornamento.
    const timer = setTimeout(() => {
      void commands.setAppMenu(nativeMenuLabels(t), nativeMenuItems()).catch(() => {});
    }, 100);
    return () => clearTimeout(timer);
  }, [enabled, registry, i18n.language, t]);
  useEffect(() => {
    const off = events.onMenuCommand((id) => runMenuCommand(id));
    return () => {
      void off.then((f) => f());
    };
  }, []);
}
