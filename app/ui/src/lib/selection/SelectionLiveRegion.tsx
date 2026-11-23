// Regione aria-live condivisa per gli annunci della selezione (unified-selection, design D3).
import { useAnnouncer } from "./announce";

/** Regione condivisa, da montare una volta nella shell. */
export function SelectionLiveRegion() {
  const { message, seq } = useAnnouncer();
  return (
    <div role="status" aria-live="polite" aria-atomic="true" className="visually-hidden">
      {/* La chiave forza la rilettura anche di un testo identico al precedente. */}
      <span key={seq}>{message}</span>
    </div>
  );
}
