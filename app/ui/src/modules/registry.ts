import type { ComponentType, SVGProps } from "react";
import { Icon } from "../components/icons";
import type { ModuleId } from "../store/layout";

type IconC = ComponentType<SVGProps<SVGSVGElement> & { size?: number }>;

/** Moduli della navigazione: per aggiungerne uno basta una voce qui e una pagina in App. */
export const MODULES: Array<{ id: ModuleId; icon: IconC; bottom?: boolean }> = [
  { id: "library", icon: Icon.Library },
  { id: "sequence", icon: Icon.Sequence },
  { id: "catalogs", icon: Icon.Catalogs },
  { id: "tags", icon: Icon.Tags },
  { id: "people", icon: Icon.People },
  { id: "identities", icon: Icon.Identities },
  { id: "import", icon: Icon.Import },
  { id: "jobs", icon: Icon.Jobs },
  { id: "models", icon: Icon.Models },
  { id: "diagnostics", icon: Icon.Diagnostics },
  { id: "organize", icon: Icon.Organize },
  { id: "settings", icon: Icon.Settings, bottom: true },
];
