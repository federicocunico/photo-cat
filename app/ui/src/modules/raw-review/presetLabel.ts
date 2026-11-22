import type { TFunction } from "i18next";
import { isRawPreview } from "./project";

/** Nome del pannello nella lingua dell'app: l'anteprima RAW ha un nome proprio. */
export function presetLabel(t: TFunction, preset: string): string {
  return isRawPreview(preset) ? t("rawReviewCompare.rawPreview") : preset;
}
