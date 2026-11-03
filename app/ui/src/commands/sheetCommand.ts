import { useTranslation } from "react-i18next";
import { useShortcutSheet } from "../store/shortcutSheet";
import { useCommands } from "./registry";

/** "?" apre il foglio delle scorciatoie. */
export function useSheetCommand() {
  const { t } = useTranslation();
  useCommands([
    {
      id: "shortcuts.open",
      label: t("commands.shortcuts"),
      group: "general",
      shortcut: { key: "?" },
      run: () => useShortcutSheet.getState().setOpen(true),
    },
  ]);
}
