import { useTranslation } from "react-i18next";
import { tagColorVar } from "../../theme/tokens";

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
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: compact ? 4 : 6,
        maxWidth: "100%",
        padding: compact ? "0 6px" : "2px 10px",
        borderRadius: 4,
        fontSize: compact ? 11 : undefined,
        background: compact ? "rgba(20,20,22,0.85)" : "var(--pc-raised)",
        color: compact ? "#E8E8EA" : undefined,
        border: compact ? 0 : "1px solid var(--pc-border)",
        borderLeft: `3px solid ${tagColorVar(color)}`,
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
      }}
    >
      {name}
    </span>
  );
}
