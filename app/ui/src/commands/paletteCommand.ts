import { useTranslation } from "react-i18next";
import { usePalette } from "../store/palette";
import { useCommands } from "./registry";

/** Ctrl+K apre la palette anche da un campo di testo. */
export function usePaletteCommand() {
  const { t } = useTranslation();
  useCommands([
    {
      id: "palette.open",
      label: t("commands.palette"),
      group: "general",
      shortcut: { key: "k", mod: true },
      menu: { menu: "go", section: 2, order: 0 },
      inInputs: true,
      palette: false,
      run: () => usePalette.getState().openPalette(),
    },
  ]);
}
