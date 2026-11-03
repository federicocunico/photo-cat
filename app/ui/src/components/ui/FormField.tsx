import { cloneElement, isValidElement, useId, type ReactElement, type ReactNode } from "react";

type ControlProps = {
  id?: string;
  "aria-describedby"?: string;
  "aria-invalid"?: boolean;
  "aria-required"?: boolean;
};

/**
 * Campo con etichetta, aiuto ed errore collegati al controllo (id, `aria-describedby`,
 * `aria-invalid`). Il controllo è l'unico figlio.
 */
export function FormField({
  label,
  hint,
  error,
  required,
  children,
  status,
}: {
  label: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  required?: boolean;
  children: ReactElement<ControlProps>;
  /** Stato accanto all'etichetta, es. la spunta "Salvato". */
  status?: ReactNode;
}) {
  const base = useId();
  const id = children.props.id ?? `${base}-control`;
  const hintId = hint ? `${base}-hint` : undefined;
  const errorId = error ? `${base}-error` : undefined;
  const describedBy =
    [children.props["aria-describedby"], hintId, errorId].filter(Boolean).join(" ") || undefined;
  const control = isValidElement(children)
    ? cloneElement(children, {
        id,
        "aria-describedby": describedBy,
        "aria-invalid": error ? true : children.props["aria-invalid"],
        "aria-required": required || children.props["aria-required"],
      })
    : children;
  return (
    <div className={["ui-field", error ? "ui-field-invalid" : ""].filter(Boolean).join(" ")}>
      <div className="ui-field-label">
        <label htmlFor={id}>
          {label}
          {required && <span aria-hidden="true"> *</span>}
        </label>
        {status && (
          <span className="ui-field-status" role="status">
            {status}
          </span>
        )}
      </div>
      {control}
      {hint && (
        <p id={hintId} className="ui-field-hint muted">
          {hint}
        </p>
      )}
      {error && (
        <p id={errorId} className="ui-field-error">
          {error}
        </p>
      )}
    </div>
  );
}
