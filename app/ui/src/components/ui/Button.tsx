import {
  forwardRef,
  useId,
  type ButtonHTMLAttributes,
  type MouseEvent,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import { BusyButton } from "../feedback/BusyButton";
import { Tooltip } from "./Tooltip";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

type BaseProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onClick"> & {
  variant?: ButtonVariant;
  size?: "sm" | "md";
  /** Icona prima del testo (decorativa). */
  icon?: ReactNode;
  /** In attesa di un'operazione avviata altrove (spinner e blocco). */
  busy?: boolean;
  /**
   * Perché il comando non è disponibile. Il pulsante resta raggiungibile da tastiera
   * (`aria-disabled`) e la ragione è letta dal lettore di schermo e mostrata al passaggio.
   */
  disabledReason?: string;
  /** Un'azione asincrona mostra lo spinner e blocca i clic ripetuti fino alla fine. */
  onClick?: (event: MouseEvent<HTMLButtonElement>) => unknown;
};

const variantClass: Record<ButtonVariant, string> = {
  primary: "btn btn-primary",
  secondary: "btn",
  ghost: "btn btn-ghost",
  danger: "btn btn-danger",
};

export const Button = forwardRef<HTMLButtonElement, BaseProps>(function Button(
  {
    variant = "secondary",
    size = "md",
    icon,
    children,
    className,
    disabledReason,
    disabled,
    type = "button",
    onClick,
    ...rest
  },
  ref,
) {
  const { t } = useTranslation();
  const reasonId = useId();
  const blocked = Boolean(disabledReason) && !disabled;
  const cls = [variantClass[variant], size === "sm" ? "btn-sm" : "", className ?? ""]
    .filter(Boolean)
    .join(" ");
  const button = (
    <BusyButton
      ref={ref}
      type={type}
      className={cls}
      disabled={disabled}
      aria-disabled={blocked || undefined}
      aria-describedby={blocked ? reasonId : rest["aria-describedby"]}
      onClick={(e) => {
        if (blocked) {
          e.preventDefault();
          return;
        }
        return onClick?.(e);
      }}
      {...rest}
    >
      {icon}
      {children}
    </BusyButton>
  );
  if (!blocked) return button;
  // La ragione è la descrizione del pulsante, non parte del suo nome.
  return (
    <>
      <Tooltip content={disabledReason!}>{button}</Tooltip>
      <span id={reasonId} className="visually-hidden">
        {t("common.disabledBecause", { reason: disabledReason })}
      </span>
    </>
  );
});

type IconButtonProps = Omit<BaseProps, "children" | "icon"> & {
  /** Nome accessibile obbligatorio: è anche il suggerimento al passaggio. */
  label: string;
  icon: ReactNode;
  /** Scorciatoia mostrata nel suggerimento, es. "Ctrl+I". */
  shortcut?: string;
};

/** Pulsante con sola icona: il nome è obbligatorio e compare come suggerimento. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { label, icon, shortcut, variant = "ghost", className, ...rest },
  ref,
) {
  const button = (
    <Button
      ref={ref}
      variant={variant}
      aria-label={label}
      className={["icon-btn", className ?? ""].filter(Boolean).join(" ")}
      {...rest}
    >
      {icon}
    </Button>
  );
  if (rest.disabledReason) return button;
  return <Tooltip content={shortcut ? `${label} (${shortcut})` : label}>{button}</Tooltip>;
});
