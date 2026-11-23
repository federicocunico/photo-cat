// Annuncio del numero di selezionati (unified-selection, design D3): una sola regione
// `aria-live` nella shell, così ogni raccolta annuncia i cambi di selezione allo stesso modo
// e i lettori di schermo non ricevono annunci doppi da più regioni.
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { create } from "zustand";

/** Genere grammaticale degli elementi: "foto" (femminile) o elementi generici (maschile). */
export type AnnounceKind = "photos" | "items";

/** Messaggio corrente della regione condivisa (letto da `SelectionLiveRegion`). */
export const useAnnouncer = create<{ message: string; seq: number }>(() => ({
  message: "",
  seq: 0,
}));

/** Annuncia un testo nella regione condivisa (ripetuto anche se uguale al precedente). */
export function announce(message: string) {
  useAnnouncer.setState((s) => ({ message, seq: s.seq + 1 }));
}

/** Annuncia il numero di selezionati quando cambia (non al primo render). */
export function useSelectionAnnouncer(count: number, kind: AnnounceKind = "items") {
  const { t } = useTranslation();
  const previous = useRef<number | null>(null);
  useEffect(() => {
    if (previous.current === null) {
      previous.current = count;
      return;
    }
    if (previous.current === count) return;
    previous.current = count;
    announce(
      count === 0 ? t("selection.announce.none") : t(`selection.announce.${kind}`, { count }),
    );
  }, [count, kind, t]);
}
