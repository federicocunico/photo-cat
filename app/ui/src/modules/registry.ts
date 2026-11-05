import type { ComponentType, SVGProps } from "react";
import { Icon } from "../components/icons";
import type { ModuleId } from "../store/layout";

type IconC = ComponentType<SVGProps<SVGSVGElement> & { size?: number }>;

export type NavGroupId = "browse" | "organize" | "ai" | "transfer" | "bottom";

export type NavItem = {
  id: ModuleId;
  icon: IconC;
  group: NavGroupId;
  /** Ctrl+N, nell'ordine dei gruppi (solo le voci dei gruppi, non quelle in fondo). */
  shortcut?: number;
};

/**
 * Voci della navigazione per gruppo (design D2): Sfoglia, Organizza, AI, Importa ed esporta,
 * poi in fondo Attività e Impostazioni. Per aggiungere un modulo basta una voce qui e una
 * pagina in App.
 */
export const NAV_GROUPS: Array<{
  id: NavGroupId;
  modules: Array<{ id: ModuleId; icon: IconC }>;
}> = [
  {
    id: "browse",
    modules: [
      { id: "library", icon: Icon.Library },
      { id: "sequence", icon: Icon.Sequence },
    ],
  },
  {
    id: "organize",
    modules: [
      { id: "catalogs", icon: Icon.Catalogs },
      { id: "people", icon: Icon.People },
      { id: "tags", icon: Icon.Tags },
    ],
  },
  {
    id: "ai",
    modules: [{ id: "ai", icon: Icon.Ai }],
  },
  {
    id: "transfer",
    modules: [
      { id: "import", icon: Icon.Import },
      { id: "organize", icon: Icon.Export },
    ],
  },
  {
    id: "bottom",
    modules: [
      { id: "jobs", icon: Icon.Jobs },
      { id: "settings", icon: Icon.Settings },
    ],
  },
];

/** Tutte le voci, nell'ordine in cui compaiono, con la scorciatoia Ctrl+1…8. */
export const MODULES: NavItem[] = (() => {
  let n = 0;
  return NAV_GROUPS.flatMap((g) =>
    g.modules.map((m) => ({
      ...m,
      group: g.id,
      shortcut: g.id === "bottom" ? undefined : ++n,
    })),
  );
})();

/** Modulo aperto da Ctrl+N, se esiste. */
export const moduleForShortcut = (n: number): ModuleId | undefined =>
  MODULES.find((m) => m.shortcut === n)?.id;
