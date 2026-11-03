import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui";
import { useGuide, type GuideSection } from "../../store/guide";
import "./guide.css";

const SECTIONS: GuideSection[] = [
  "library",
  "people",
  "catalogs",
  "tags",
  "sequence",
  "import",
  "settings",
];

/** Aiuto › Guida, e "Scopri di più" dei suggerimenti (aperta sulla sezione del modulo). */
export function GuideDialog() {
  const { t } = useTranslation();
  const section = useGuide((s) => s.guide);
  const openGuide = useGuide((s) => s.openGuide);
  useEffect(() => {
    if (!section) return;
    // Dopo il montaggio del dialogo: porta in vista la sezione richiesta.
    const timer = setTimeout(
      () => document.getElementById(`guide-${section}`)?.scrollIntoView?.({ block: "start" }),
      0,
    );
    return () => clearTimeout(timer);
  }, [section]);
  return (
    <Dialog
      open={section != null}
      onOpenChange={(open) => !open && openGuide(null)}
      title={t("guide.title")}
      width={640}
    >
      <p className="muted">{t("guide.intro")}</p>
      <div className="guide-sections">
        {SECTIONS.map((s) => (
          <section
            key={s}
            id={`guide-${s}`}
            aria-labelledby={`guide-${s}-title`}
            data-current={s === section || undefined}
          >
            <h3 id={`guide-${s}-title`}>{t(`guide.sections.${s}.title`)}</h3>
            <p>{t(`guide.sections.${s}.body`)}</p>
          </section>
        ))}
      </div>
    </Dialog>
  );
}
