import type { ReactNode, Ref } from "react";
import { PageHeader } from "../ui";

/**
 * Pagina di un modulo (desktop-shell, "Intestazione di pagina" e "Scorrimento e barra di
 * stato"): intestazione con il titolo della voce di navigazione, sottotitolo, schede e comandi,
 * sempre nella stessa posizione e con la stessa altezza; sotto, il corpo del modulo, che è
 * l'unico contenitore che scorre.
 */
export function ModulePage({
  title,
  subtitle,
  tabs,
  actions,
  scroll = true,
  className,
  bodyClassName,
  bodyRef,
  children,
}: {
  title: ReactNode;
  /** Testo breve accanto al titolo (es. la fotocamera supportata). */
  subtitle?: ReactNode;
  /** Schede del modulo, accanto al titolo. */
  tabs?: ReactNode;
  /** Comandi della pagina, a destra. */
  actions?: ReactNode;
  /**
   * `true`: il corpo scorre (pagine di moduli e impostazioni). `false`: il modulo gestisce da sé
   * lo scorrimento dei suoi riquadri (elenchi virtualizzati, colonne).
   */
  scroll?: boolean;
  className?: string;
  /** Classe del corpo: impaginazione propria del modulo (spaziature, colonne). */
  bodyClassName?: string;
  bodyRef?: Ref<HTMLDivElement>;
  children: ReactNode;
}) {
  return (
    <div className={["module-page", className].filter(Boolean).join(" ")}>
      <PageHeader title={title} description={subtitle} tabs={tabs} actions={actions} />
      <div
        ref={bodyRef}
        className={["module-body", scroll ? "module-body--scroll" : "", bodyClassName ?? ""]
          .filter(Boolean)
          .join(" ")}
      >
        {children}
      </div>
    </div>
  );
}
