// Nome mostrato delle voci e dei gruppi di catalogazione: traduzione per le
// voci predefinite non rinominate, altrimenti il nome scritto nella tassonomia
// (voci create o rinominate dall'utente), e come ultima risorsa l'id.
import { useQuery } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import i18n from "../../i18n";
import { commands, isTauri, type Taxonomy } from "../../lib/ipc";

export function useTaxonomy() {
  return useQuery({ queryKey: ["taxonomy"], queryFn: commands.getTaxonomy, enabled: isTauri() });
}

/** Nome predefinito (tassonomia di fabbrica, in italiano) di una chiave tradotta. */
const factory = (key: string) =>
  i18n.exists(key, { lng: "it" }) ? i18n.getFixedT("it")(key) : undefined;

const translated = (t: TFunction, key: string, saved: string | undefined) => {
  const base = factory(key);
  if (saved === undefined) return base !== undefined ? t(key) : undefined;
  return base !== undefined && saved === base ? t(key) : saved;
};

export function labelNameOf(
  tax: Taxonomy | undefined,
  t: TFunction,
  group: string,
  value: string,
): string {
  const category = tax?.groups.find((g) => g.id === group)?.categories.find((c) => c.id === value);
  return translated(t, `labels.${value}`, category?.label) ?? value.replace(/_/g, " ");
}

export function groupNameOf(tax: Taxonomy | undefined, t: TFunction, group: string): string {
  const saved = tax?.groups.find((g) => g.id === group)?.label;
  return translated(t, `library.facets.groups.${group}`, saved) ?? group.replace(/_/g, " ");
}

export function useLabelName() {
  const { t } = useTranslation();
  const tax = useTaxonomy().data;
  return {
    label: (group: string, value: string) => labelNameOf(tax, t, group, value),
    group: (group: string) => groupNameOf(tax, t, group),
  };
}
