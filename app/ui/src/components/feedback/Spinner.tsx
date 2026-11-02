import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import "./feedback.css";

export function Spinner({ size = 16, label }: { size?: number; label?: string }) {
  const { t } = useTranslation();
  return (
    <span role="status" aria-label={label ?? t("feedback.loading")} className="feedback-spinner">
      <Icon.Spinner size={size} />
    </span>
  );
}
