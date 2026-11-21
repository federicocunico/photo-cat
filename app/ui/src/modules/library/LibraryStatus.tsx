import { useTranslation } from "react-i18next";
import { useLibraryCount } from "../../lib/latest";
import { fmtInt } from "../../lib/format";
import { selCount, useLibrary } from "../../store/library";

/** Conteggi della Libreria nella barra di stato. */
export function LibraryStatus() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const selected = useLibrary((s) => selCount(s.selection));
  const count = useLibraryCount(filter);
  return (
    <>
      <span data-testid="status-count">
        {count.isPending
          ? t("feedback.calculating")
          : count.isError
            ? t("feedback.error")
            : t("library.count", { n: fmtInt(count.data) })}
      </span>
      {/* aria-live: il nuovo conteggio (Ctrl+A, Inverti, Esc) è annunciato dai lettori di schermo. */}
      <span data-testid="status-selected" aria-live="polite" aria-atomic="true">
        {t("library.selected", { n: fmtInt(selected) })}
      </span>
    </>
  );
}
