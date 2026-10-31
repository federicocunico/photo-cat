import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLibrary } from "../../store/library";

/** Conteggi della Libreria nella barra di stato. */
export function LibraryStatus() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const selected = useLibrary((s) => s.selected.length);
  const count = useQuery({
    queryKey: ["library-count", filter],
    queryFn: () => commands.libraryCount(filter),
    enabled: isTauri(),
  });
  return (
    <>
      <span data-testid="status-count">{t("library.count", { n: fmtInt(count.data ?? 0) })}</span>
      <span data-testid="status-selected">{t("library.selected", { n: fmtInt(selected) })}</span>
    </>
  );
}
