import { useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";

/**
 * Modifica sul campo (design D5): il valore è un pulsante; un clic o Invio apre
 * l'editor, Esc lo chiude senza modifiche e il focus torna sul valore.
 */
export function InlineEdit({
  label,
  value,
  placeholder,
  children,
}: {
  /** Nome del campo, es. "Luogo". */
  label: string;
  /** Valore attuale in parole; vuoto mostra `placeholder`. */
  value: string | null;
  placeholder?: string;
  /** Editor; `close` va chiamato dopo aver applicato la modifica. */
  children: (close: () => void) => ReactNode;
}) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const close = () => {
    setEditing(false);
    requestAnimationFrame(() => button.current?.focus());
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    e.preventDefault();
    e.stopPropagation();
    close();
  };
  if (editing)
    return (
      <div className="ui-inline-editor" role="group" aria-label={label} onKeyDown={onKeyDown}>
        {children(close)}
      </div>
    );
  const shown = value || placeholder || t("common.notSet");
  return (
    <button
      ref={button}
      type="button"
      className="ui-inline-value"
      data-empty={!value || undefined}
      aria-label={t("common.editField", { field: label, value: shown })}
      onClick={() => setEditing(true)}
    >
      <span>{shown}</span>
      <Icon.Edit size={12} aria-hidden="true" className="ui-inline-icon" />
    </button>
  );
}
