// Azione sul luogo (job bulk "set_place"): esito e "Annulla operazione" nell'unico
// ToastHost, viste aggiornate al termine (useBulkJob).
import { useTranslation } from "react-i18next";
import type { PlaceInput, Target } from "../../lib/ipc";
import { useBulkJob } from "./bulkJob";

export function usePlaceAction(onDone?: () => void) {
  const { t } = useTranslation();
  const bulk = useBulkJob(() => onDone?.());
  const apply = async (target: Target, place: PlaceInput | null, total?: number) => {
    const count = total ?? (target.kind === "ids" ? target.ids.length : 1);
    const message = place
      ? t("selection.done.set_place", { name: place.name, count })
      : t("selection.done.reset_place", { count });
    await bulk.start(target, { action: "set_place", place }, message);
  };
  // Gli errori arrivano come avvisi: il campo resta per i chiamanti esistenti.
  return { apply, error: "", busy: bulk.busy };
}
