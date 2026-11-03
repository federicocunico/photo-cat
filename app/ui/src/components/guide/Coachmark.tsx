import * as RPopover from "@radix-ui/react-popover";
import { useEffect, useId, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui";
import { useGuide, type GuideSection, type TipId } from "../../store/guide";
import "./guide.css";

/**
 * Suggerimento al primo uso: ancorato a un elemento, non modale (non prende il focus e
 * non blocca il resto della pagina), al massimo uno visibile alla volta. Si chiude con
 * "Ho capito" o Esc e non ricompare, salvo Aiuto › "Mostra di nuovo i suggerimenti".
 */
export function Coachmark({
  tip,
  anchor,
  side = "bottom",
  learnMore,
}: {
  tip: TipId;
  anchor: RefObject<HTMLElement | null>;
  side?: "top" | "right" | "bottom" | "left";
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
    <RPopover.Root open modal={false} onOpenChange={(open) => !open && dismiss()}>
      <RPopover.Anchor virtualRef={anchor as RefObject<HTMLElement>} />
      <RPopover.Portal>
        <RPopover.Content
          className="ui-popover ui-coachmark"
          side={side}
          sideOffset={8}
          collisionPadding={8}
          aria-labelledby={id}
          // Non modale: il focus resta dov'è e un clic altrove non lo chiude.
          onOpenAutoFocus={(e) => e.preventDefault()}
          onCloseAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => e.preventDefault()}
          data-tip={tip}
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
          <RPopover.Arrow className="ui-coachmark-arrow" width={12} height={6} />
        </RPopover.Content>
      </RPopover.Portal>
    </RPopover.Root>
  );
}
