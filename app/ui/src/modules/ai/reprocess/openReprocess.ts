// Apre la sezione Rianalisi con la selezione indicata come ambito.
import type { Target } from "../../../lib/ipc";
import { useLayout } from "../../../store/layout";
import { useReprocessPreset } from "./preset";

export function openReprocessFor(target: Target, count?: number | null) {
  useReprocessPreset.getState().set({ scope: { kind: "target", target }, count });
  useLayout.getState().openModule({ module: "ai", section: "reprocess" });
}
