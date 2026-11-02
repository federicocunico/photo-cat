import { useTranslation } from "react-i18next";
import "./feedback.css";

export function RefreshBar({ active }: { active: boolean }) {
  const { t } = useTranslation();
  return active ? (
    <div role="status" aria-label={t("feedback.refreshing")} className="feedback-refresh">
      <span />
    </div>
  ) : null;
}
