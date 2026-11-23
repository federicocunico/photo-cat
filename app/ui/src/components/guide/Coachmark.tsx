import { useEffect, useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui";
import { useGuide, type GuideSection, type TipId } from "../../store/guide";
import "./guide.css";

/**
 * Suggerimento al primo uso (in-app-guidance; ui-usability-audit D12): una nota in linea, nel
 * flusso della pagina, accanto al titolo o all'elenco a cui si riferisce. Non è un popover:
 * non copre comandi né contenuto, non prende il focus e non blocca nulla. Al massimo uno visibile
 * alla volta; si chiude con "Ho capito" e non ricompare, salvo Aiuto › "Mostra di nuovo i
 * suggerimenti".
 */
export function Coachmark({
  tip,
  placement = "inline-start",
  learnMore,
}: {
  tip: TipId;
  /** Allineamento nella riga che lo ospita: all'inizio (sinistra) o alla fine (destra). */
  placement?: "inline-start" | "inline-end";
  /** Sezione della guida aperta da "Scopri di più". */
  learnMore?: GuideSection;
}) {
  const { t } = useTranslation();
  const id = useId();
  const visible = useGuide((s) => s.active === tip);
  // Quando non c'è un suggerimento aperto (anche dopo "Mostra di nuovo i suggerimenti")
  // questo prova a comparire.
  const free = useGuide((s) => s.active === null);
  useEffect(() => {
    useGuide.getState().claim(tip);
    return () => useGuide.getState().release(tip);
  }, [tip, free]);
  if (!visible) return null;
  const dismiss = () => useGuide.getState().dismiss(tip);
  return (
    <aside
      role="note"
      aria-labelledby={id}
      className="ui-coachmark"
      data-placement={placement}
      data-tip={tip}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          dismiss();
        }
      }}
    >
      <p id={id}>{t(`guide.tips.${tip}`)}</p>
      <div className="ui-coachmark-actions">
        {learnMore && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              dismiss();
              useGuide.getState().openGuide(learnMore);
            }}
          >
            {t("guide.learnMore")}
          </Button>
        )}
        <Button variant="primary" size="sm" onClick={dismiss}>
          {t("guide.gotIt")}
        </Button>
      </div>
    </aside>
  );
}
