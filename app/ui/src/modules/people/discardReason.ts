import type { DiscardReason } from "../../lib/ipc";

/** Tooltip del motivo di scarto qualità (mockup: «sfocato», «troppo piccolo: 28 px», …). */
export function discardReasonLabel(
  reason: DiscardReason | null,
  sizePx: number | null,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string | undefined {
  if (!reason) return undefined;
  switch (reason) {
    case "blurred":
      return t("people.discard.blurred");
    case "too_small":
      return t("people.discard.tooSmall", { px: sizePx ?? 0 });
    case "non_frontal":
      return t("people.discard.nonFrontal");
    case "low_confidence":
      return t("people.discard.lowConfidence");
  }
}
