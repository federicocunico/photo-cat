import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useCommands } from "../../commands/registry";
import { commands, isTauri } from "../../lib/ipc";
import { emptyFilter, useLibrary } from "../../store/library";
import { collectDuplicates, startQualityCheck, systemCatalogId } from "../catalogs/systemCatalogs";

/** Comandi della palette per i cataloghi di sistema (Cestino, Duplicati, Qualità). */
export function useSystemCatalogCommands() {
  const { t } = useTranslation();
  const qc = useQueryClient();
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
    {
      id: "system.collectDuplicates",
      label: t("catalogs.system.collectDuplicates"),
      group: "library",
      run: () => collectDuplicates(qc, t),
    },
    {
      id: "system.checkQuality",
      label: t("catalogs.system.checkQuality"),
      group: "library",
      run: () => startQualityCheck(t),
    },
  ]);
}
