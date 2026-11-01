import { useTranslation } from "react-i18next";
import { activeSet, keyRange, useTagOverview } from "./data";

/** Set attivo e tasti disponibili, nella barra di stato di Libreria, Tag e Sequenza. */
export function ActiveSetStatus() {
  const { t } = useTranslation();
  const set = activeSet(useTagOverview().data);
  if (!set) return null;
  const keys = set.tags.map((x) => x.key);
  return (
    <span data-testid="active-set">
      {keys.length
        ? t("tags.activeStatus", { name: set.name, keys: keyRange(keys) })
        : t("tags.activeStatusEmpty", { name: set.name })}
    </span>
  );
}

/** Conteggi della schermata Tag. */
export function TagsStatus() {
  const { t } = useTranslation();
  const o = useTagOverview().data;
  return (
    <>
      <span>{t("tags.status", { tags: o?.tags.length ?? 0, sets: o?.sets.length ?? 0 })}</span>
      <ActiveSetStatus />
    </>
  );
}
