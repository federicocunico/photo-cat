import { useTranslation } from "react-i18next";
import type { MenuItem } from "../../components/ui";
import { commands } from "../../lib/ipc";
import { selCount, useLibrary } from "../../store/library";
import { notifyError } from "../../store/toasts";
import { useLibrarySelection } from "./useLibrarySelection";

/** Seleziona tutto, inverti, foto non principali dei simili: senza e con selezione. */
export function useSelectItems(): MenuItem[] {
  const { t } = useTranslation();
  const { filter, selection, setSelection } = useLibrary();
  const grid = useLibrarySelection();
  const count = selCount(selection);
  const run = (fn: () => Promise<unknown>) => () => fn().catch((e) => notifyError(e));
  return [
    {
      label: t("selection.selectAll"),
      shortcut: "Ctrl+A",
      onSelect: () => void grid.selectAll(),
    },
    {
      label: t("selection.invert"),
      onSelect: () => void grid.invert(),
    },
    {
      label: t("similar.nonCovers"),
      onSelect: run(async () => {
        // Sulla selezione esplicita, altrimenti su tutti i risultati del filtro.
        const explicit = selection.mode === "ids" && count > 0 ? [...selection.ids] : null;
        const ids = await commands.similarNonCovers(
          explicit ? { ...filter, ids: explicit } : filter,
        );
        setSelection(ids, ids[0] ?? null);
      }),
    },
  ];
}
