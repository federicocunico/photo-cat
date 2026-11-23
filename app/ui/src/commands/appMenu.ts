// Comandi globali della barra dei menu (application-menu, D6): quelli che non appartengono a un
// modulo montato. Le voci dei moduli non montati aprono il modulo e poi l'azione; quelle che
// richiedono una selezione restano visibili, disattivate, con il motivo.
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { commands, isTauri } from "../lib/ipc";
import { copySystemInfo } from "../lib/systemInfo";
import { selCount, useLibrary } from "../store/library";
import { useLayout } from "../store/layout";
import { useGuide } from "../store/guide";
import { useAboutDialog, useMenuIntent } from "../store/menuIntent";
import { useSelectionPicker } from "../store/palette";
import { notify, notifyError } from "../store/toasts";
import { collectDuplicates, startQualityCheck } from "../modules/catalogs/systemCatalogs";
import type { ThemePref } from "../theme/tokens";
import { useCommands, type Command } from "./registry";

const librarySelection = () => selCount(useLibrary.getState().selection);

export function useAppMenuCommands() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const openModule = useLayout((s) => s.openModule);
  const needsSelection = () => (librarySelection() > 0 ? undefined : t("menu.why.selectPhotos"));
  const desktopOnly = () => (isTauri() ? undefined : t("menu.why.desktopOnly"));

  const themes: ThemePref[] = ["system", "dark", "light"];
  const commandsList: Command[] = [
    // File
    {
      id: "file.import",
      label: t("menu.importFolder"),
      group: "general",
      shortcut: { key: "o", mod: true },
      menu: { menu: "file", section: 0, order: 0 },
      run: () => {
        openModule({ module: "import" });
        useMenuIntent.getState().request("import.pick");
      },
    },
    {
      id: "file.export",
      label: t("menu.export"),
      group: "general",
      shortcut: { key: "e", mod: true },
      menu: { menu: "file", section: 0, order: 1 },
      run: () => openModule({ module: "organize" }),
    },
    {
      id: "file.openProject",
      label: t("menu.openProject"),
      group: "general",
      keywords: "FujiView RAW Fujifilm X-T5",
      menu: { menu: "file", section: 1, order: 1 },
      run: () => {
        openModule({ module: "raw-review" });
        useMenuIntent.getState().request("rawReview.project");
      },
    },
    {
      id: "file.showDataDir",
      label: t("menu.showDataDir"),
      group: "general",
      menu: { menu: "file", section: 2, order: 0 },
      disabledReason: desktopOnly,
      run: () => commands.openDataDir().catch(notifyError),
    },
    {
      id: "file.quit",
      label: t("menu.quit"),
      group: "general",
      menu: { menu: "file", section: 3, order: 0 },
      disabledReason: desktopOnly,
      // Come la chiusura della finestra: il backend conclude le attività in corso.
      run: () => getCurrentWindow().close(),
    },
    // Modifica
    {
      id: "edit.addToCatalog",
      label: t("menu.addToCatalog"),
      group: "selection",
      menu: { menu: "edit", section: 3, order: 0 },
      disabledReason: needsSelection,
      run: () => {
        openModule({ module: "library" });
        useSelectionPicker.getState().set("catalog");
      },
    },
    {
      id: "edit.moveToTrash",
      label: t("menu.moveToTrash"),
      group: "selection",
      menu: { menu: "edit", section: 3, order: 1 },
      disabledReason: needsSelection,
      run: () => {
        openModule({ module: "library" });
        useSelectionPicker.getState().set("trash");
      },
    },
    // Segnaposto delle voci di Libreria quando la Libreria non è aperta.
    {
      id: "fallback.tags",
      label: t("menu.activeSetTags"),
      group: "selection",
      fallbackFor: "tags.key.",
      menu: { menu: "edit", section: 2, order: 0 },
      disabledReason: () => t("menu.why.library"),
    },
    {
      id: "fallback.rating",
      label: t("menu.rating"),
      group: "selection",
      fallbackFor: "rating.key.",
      menu: { menu: "edit", section: 2, order: 20 },
      disabledReason: () => t("menu.why.library"),
    },
    {
      id: "fallback.panels.sides",
      label: t("menu.sidePanels"),
      group: "library",
      shortcut: { key: "b", mod: true, shift: true },
      fallbackFor: "panels.sides",
      menu: { menu: "view", section: 0, order: 1 },
      disabledReason: () => t("menu.why.library"),
    },
    {
      id: "fallback.panels.inspector",
      label: t("menu.inspector"),
      group: "library",
      shortcut: { key: "i", mod: true },
      fallbackFor: "panels.inspector",
      menu: { menu: "view", section: 0, order: 2 },
      disabledReason: () => t("menu.why.library"),
    },
    // Vista
    {
      id: "view.groupSimilar",
      label: t("menu.groupSimilar"),
      group: "library",
      menu: { menu: "view", section: 1, order: 1 },
      checked: () => useLibrary.getState().groupSimilar,
      run: () => {
        const lib = useLibrary.getState();
        lib.setGroupSimilar(!lib.groupSimilar);
      },
    },
    ...[1, 2, 3, 4, 5].map((n): Command => ({
      id: `view.thumbSize.${n}`,
      label: n === 5 ? t("menu.thumbSizeLargest") : t("menu.thumbSizeValue", { count: n }),
      group: "library",
      keywords: t("menu.thumbSize"),
      menu: { menu: "view", section: 1, order: 0, submenu: t("menu.thumbSize") },
      checked: () => useLibrary.getState().thumbSize === n,
      run: () => useLibrary.getState().setThumbSize(n),
    })),
    ...themes.map((theme, i): Command => ({
      id: `view.theme.${theme}`,
      label: t(`settings.${theme}`),
      group: "general",
      keywords: t("menu.theme"),
      menu: { menu: "view", section: 3, order: i, submenu: t("menu.theme") },
      checked: () => useLayout.getState().theme === theme,
      run: () => useLayout.getState().setTheme(theme),
    })),
    {
      id: "view.fullscreen",
      label: t("menu.fullscreen"),
      group: "general",
      shortcut: { key: "F11" },
      menu: { menu: "view", section: 3, order: 3 },
      disabledReason: desktopOnly,
      run: async () => {
        const win = getCurrentWindow();
        await win.setFullscreen(!(await win.isFullscreen()));
      },
    },
    // Strumenti
    {
      id: "system.collectDuplicates",
      label: t("menu.collectDuplicates"),
      group: "library",
      menu: { menu: "tools", section: 0, order: 1 },
      run: () => collectDuplicates(qc, t),
    },
    {
      id: "system.checkQuality",
      label: t("menu.checkQuality"),
      group: "library",
      menu: { menu: "tools", section: 0, order: 2 },
      run: () => startQualityCheck(t),
    },
    {
      id: "tools.models",
      label: t("menu.models"),
      group: "navigation",
      menu: { menu: "tools", section: 1, order: 0 },
      run: () => openModule({ module: "settings", section: "models" }),
    },
    {
      id: "tools.diagnostics",
      label: t("menu.diagnostics"),
      group: "navigation",
      keywords: "log",
      menu: { menu: "tools", section: 1, order: 1 },
      run: () => openModule({ module: "settings", section: "diagnostics" }),
    },
    // Aiuto
    {
      id: "help.guide",
      label: t("menu.guide"),
      group: "general",
      menu: { menu: "help", section: 0, order: 0 },
      run: () => useGuide.getState().openGuide("library"),
    },
    {
      id: "help.firstSteps",
      label: t("menu.firstSteps"),
      group: "general",
      menu: { menu: "help", section: 0, order: 2 },
      run: () => {
        useGuide.getState().show();
        openModule({ module: "library" });
      },
    },
    {
      id: "help.intro",
      label: t("menu.intro"),
      group: "general",
      menu: { menu: "help", section: 0, order: 3 },
      run: () => useGuide.getState().setIntro(true),
    },
    {
      id: "help.tips",
      label: t("menu.tips"),
      group: "general",
      menu: { menu: "help", section: 0, order: 4 },
      run: () => {
        useGuide.getState().resetTips();
        notify({ tone: "ok", message: t("guide.tipsReset") });
      },
    },
    {
      id: "help.report",
      label: t("menu.report"),
      group: "general",
      menu: { menu: "help", section: 1, order: 0 },
      run: () => {
        useGuide.getState().setReport(true);
        openModule({ module: "settings", section: "diagnostics" });
      },
    },
    {
      id: "help.copyInfo",
      label: t("menu.copyInfo"),
      group: "general",
      menu: { menu: "help", section: 1, order: 1 },
      run: () => copySystemInfo(t),
    },
    {
      id: "help.about",
      label: t("menu.about"),
      group: "general",
      keywords: "versione build licenze",
      menu: { menu: "help", section: 2, order: 0 },
      run: () => useAboutDialog.getState().setOpen(true),
    },
  ];
  useCommands(commandsList);
}
