// Modulo "Varianti RAW (Fujifilm X-T5)": per ora solo il titolo e lo stato vuoto.
// La scelta della cartella, le sessioni recenti e il confronto arrivano con i task 3.2–3.4.
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { EmptyState, PageHeader } from "../../components/ui";
import "../../components/shell/shell.css";

export function RawReviewPage() {
  const { t } = useTranslation();
  return (
    <div className="module-placeholder">
      <PageHeader title={t("modules.raw-review")} description={t("rawReviewPage.subtitle")} />
      <EmptyState
        icon={<Icon.RawReview size={32} aria-hidden="true" />}
        title={t("rawReviewPage.emptyTitle")}
        description={t("rawReviewPage.emptyHint")}
      />
    </div>
  );
}
