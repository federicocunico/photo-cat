import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useCommands } from "../../commands/registry";
import { commands, isTauri } from "../../lib/ipc";
import { emptyFilter, useLibrary } from "../../store/library";
import { systemCatalogId } from "../catalogs/systemCatalogs";

/** Comando della palette per aprire il Cestino (Duplicati e Qualità sono nel menu Strumenti). */
export function useSystemCatalogCommands() {
  const { t } = useTranslation();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const open = (role: "trash" | "duplicates" | "low_quality") => {
    const id = systemCatalogId(catalogs.data, role);
    if (id == null) return;
    const lib = useLibrary.getState();
    lib.setSelection([], null);
    lib.setFilter({ ...emptyFilter(), catalogId: id });
  };
  useCommands([
    {
      id: "system.openTrash",
      label: t("catalogs.system.openTrash"),
      group: "library",
      run: () => open("trash"),
    },
    // "Metti da parte i duplicati" e "Controlla la qualità" sono comandi globali (menu Strumenti).
  ]);
}
