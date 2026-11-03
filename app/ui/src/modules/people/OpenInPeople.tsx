import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { useLayout, type PeopleTarget } from "../../store/layout";

/** Pulsante condiviso: apre Persone sulla persona/volto indicato. */
export function OpenInPeople({
  target,
  className = "btn btn-ghost",
  style,
}: {
  target: PeopleTarget;
  className?: string;
  style?: CSSProperties;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className={className}
      style={style}
      onClick={() => useLayout.getState().openModule({ module: "people", target: target })}
    >
      {t("identities.openPeople")}
    </button>
  );
}
