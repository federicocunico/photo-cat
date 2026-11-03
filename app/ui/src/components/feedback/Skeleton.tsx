import { useTranslation } from "react-i18next";
import "./feedback.css";

export function Skeleton({
  kind = "row",
  count = 3,
}: {
  kind?: "row" | "block" | "thumbnail";
  count?: number;
}) {
  const { t } = useTranslation();
  return (
    <div
      role="status"
      aria-label={t("feedback.loading")}
      className={`feedback-skeletons feedback-skeletons-${kind}`}
    >
      {Array.from({ length: count }, (_, i) => (
        <span
          key={i}
          aria-hidden="true"
          className={`feedback-skeleton feedback-skeleton-${kind}`}
        />
      ))}
    </div>
  );
}
