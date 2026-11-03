// "Aggiungi a catalogo" condiviso da menu contestuale e trascinamento: avvia l'azione bulk
// `add_catalog` con avviso d'esito e "Annulla operazione".
import { useTranslation } from "react-i18next";
import type { Target } from "../../lib/ipc";
import { useBulkJob } from "./bulkJob";

export function useAddToCatalog() {
  const { t } = useTranslation();
  const bulk = useBulkJob();
  return (target: Target, count: number, catalog: number, name: string) =>
    void bulk.start(
      target,
      { action: "add_catalog", catalog },
      t("selection.done.add_catalog", { count, name }),
    );
}
