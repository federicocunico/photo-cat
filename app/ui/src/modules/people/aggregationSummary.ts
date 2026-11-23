import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { notify } from "../../store/toasts";

/**
 * Riepilogo della prima riaggregazione dopo l'aggiornamento (face-identity-aggregation, D7):
 * "52 gruppi uniti automaticamente", con il collegamento a "Unioni recenti", dove ogni unione si
 * annulla. Compare una sola volta: appena mostrato si segna come visto nel backend.
 */
export function useAggregationSummary(openMerges: () => void) {
  const { t } = useTranslation();
  const shown = useRef(false);
  const open = useRef(openMerges);
  open.current = openMerges;
  const summary = useQuery({
    queryKey: ["aggregation-summary"],
    queryFn: commands.aggregationSummary,
    enabled: isTauri(),
    staleTime: Infinity,
  });
  const data = summary.data;
  useEffect(() => {
    if (!data || shown.current) return;
    shown.current = true;
    void commands.dismissAggregationSummary().catch(() => undefined);
    if (data.autoMerges <= 0) return;
    notify({
      key: "aggregation-summary",
      tone: "info",
      duration: null,
      message: t("people.merges.summary", { count: data.autoMerges }),
      action: { label: t("people.merges.summaryAction"), run: () => open.current() },
    });
  }, [data, t]);
}
