// Componenti di impaginazione e stato: intestazione di pagina, scheda, avvisi in linea,
// stato vuoto, badge e tasti.
import type { HTMLAttributes, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";

/** Intestazione di pagina: il titolo è uguale alla voce di navigazione del modulo. */
export function PageHeader({
  title,
  description,
  actions,
  breadcrumb,
}: {
  title: ReactNode;
  description?: ReactNode;
  /** Comandi principali della pagina, allineati a destra. */
  actions?: ReactNode;
  /** Percorso sopra il titolo (es. Tutte le foto › Catalogo). */
  breadcrumb?: ReactNode;
}) {
  return (
    <header className="ui-page-header">
      <div className="ui-page-header-text">
        {breadcrumb && <nav className="ui-breadcrumb muted">{breadcrumb}</nav>}
        <h1>{title}</h1>
        {description && <p className="muted">{description}</p>}
      </div>
      {actions && <div className="ui-page-header-actions">{actions}</div>}
    </header>
  );
}

/** Riquadro con titolo facoltativo: una sezione della pagina. */
export function Card({
  title,
  actions,
  children,
  className,
  ...rest
}: Omit<HTMLAttributes<HTMLElement>, "title"> & { title?: ReactNode; actions?: ReactNode }) {
  return (
    <section className={["ui-card", className ?? ""].filter(Boolean).join(" ")} {...rest}>
      {(title || actions) && (
        <div className="ui-card-head">
          {title && <h2>{title}</h2>}
          {actions}
        </div>
      )}
      {children}
    </section>
  );
}

export type Tone = "info" | "ok" | "warn" | "danger";

const toneIcon: Record<Tone, ReactNode> = {
  info: <Icon.Info size={16} />,
  ok: <Icon.Check size={16} />,
  warn: <Icon.Alert size={16} />,
  danger: <Icon.Alert size={16} />,
};

/**
 * Messaggio in linea vicino al punto a cui si riferisce. Gli errori sono annunciati
 * subito (`role="alert"`), gli altri con cortesia (`role="status"`). Il tono è dato
 * anche da icona e testo, non solo dal colore.
 */
export function InlineAlert({
  tone = "info",
  title,
  children,
  action,
  onDismiss,
}: {
  tone?: Tone;
  title?: ReactNode;
  children?: ReactNode;
  action?: ReactNode;
  onDismiss?: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className={`ui-alert ui-alert-${tone}`} role={tone === "danger" ? "alert" : "status"}>
      <span className="ui-alert-icon">{toneIcon[tone]}</span>
      <div className="ui-alert-body">
        {title && <strong>{title}</strong>}
        {children && <div>{children}</div>}
      </div>
      {action && <div className="ui-alert-action">{action}</div>}
      {onDismiss && (
        <button
          type="button"
          className="btn btn-ghost icon-btn"
          aria-label={t("common.dismiss")}
          onClick={onDismiss}
        >
          <Icon.Close size={14} />
        </button>
      )}
    </div>
  );
}

/** Stato vuoto: cosa manca, perché, e il passo successivo. */
export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="ui-empty">
      {icon && <div className="ui-empty-icon">{icon}</div>}
      <h2>{title}</h2>
      {description && <p className="muted">{description}</p>}
      {action && <div className="ui-empty-action">{action}</div>}
    </div>
  );
}

/** Conteggio o etichetta breve. Con `label` il numero ha un nome per il lettore di schermo. */
export function Badge({
  children,
  tone = "info",
  label,
}: {
  children: ReactNode;
  tone?: Tone | "neutral";
  label?: string;
}) {
  return (
    <span
      className={`ui-badge ui-badge-${tone}`}
      aria-label={label}
      role={label ? "img" : undefined}
    >
      {children}
    </span>
  );
}

/** Combinazione di tasti, es. `<Kbd keys={["Ctrl", "K"]} />`. */
export function Kbd({ keys }: { keys: string[] }) {
  return (
    <span className="ui-kbd-group">
      {keys.map((k, i) => (
        <kbd key={i} className="ui-kbd">
          {k}
        </kbd>
      ))}
    </span>
  );
}
