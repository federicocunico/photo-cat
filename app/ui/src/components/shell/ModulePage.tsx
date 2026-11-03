import type { ReactNode } from "react";
import { PageHeader } from "../ui";

/**
 * Pagina di un modulo: intestazione con il titolo della voce di navigazione, poi il
 * contenuto, che occupa lo spazio restante (desktop-shell, "Stile visivo").
 */
export function ModulePage({
  title,
  description,
  actions,
  children,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="module-page">
      <PageHeader title={title} description={description} actions={actions} />
      {children}
    </div>
  );
}
