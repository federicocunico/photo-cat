// Scelta della variante migliore da un pannello (clic, Invio, Maiusc+1–9): l'"Anteprima RAW"
// si valuta ma non si sceglie, perché non è un JPEG da esportare.
import type { TFunction } from "i18next";
import type { ReviewShotView } from "../../lib/ipc";
import { notify } from "../../store/toasts";

/** Vero se `preset` si può scegliere per lo scatto; altrimenti avvisa (se la variante c'è). */
export function canChoose(shot: ReviewShotView, preset: string, t: TFunction): boolean {
  const variant = shot.variants.find((v) => v.preset === preset);
  if (!variant) return false;
  if (!variant.selectable) {
    notify({
      key: "raw-review-not-selectable",
      tone: "info",
      message: t("rawReviewCompare.rawNotChoosable"),
    });
    return false;
  }
  return true;
}
