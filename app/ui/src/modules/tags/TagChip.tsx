import { useTranslation } from "react-i18next";
import { tagColorVar } from "../../theme/tokens";
import "./tags.css";

/** Chip di un tag con il suo colore (Libreria, Inspector, Sequenza). */
export function TagChip({
  name,
  color,
  compact = false,
}: {
  name: string;
  color: string;
  compact?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <span
      data-tag-color={color}
      aria-label={t("tags.chip", { name })}
      className={compact ? "tag-chip tag-chip-compact" : "tag-chip"}
      style={{ borderLeft: `3px solid ${tagColorVar(color)}` }}
    >
      {name}
    </span>
  );
}
