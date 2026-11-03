import { forwardRef, useRef, useState, type ButtonHTMLAttributes, type MouseEvent } from "react";
import { Spinner } from "./Spinner";
import { notify } from "../../store/toasts";

type Props = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "onClick"> & {
  onClick?: (event: MouseEvent<HTMLButtonElement>) => unknown;
  busy?: boolean;
};

export const BusyButton = forwardRef<HTMLButtonElement, Props>(function BusyButton(
  { children, onClick, disabled, busy = false, ...props },
  ref,
) {
  const locked = useRef(false);
  const [pending, setPending] = useState(false);
  return (
    <button
      ref={ref}
      {...props}
      disabled={disabled || busy}
      aria-disabled={pending || undefined}
      aria-busy={busy || pending || undefined}
      onClick={(event) => {
        if (locked.current || disabled || busy || pending) {
          event.preventDefault();
          return;
        }
        locked.current = true;
        try {
          const result = onClick?.(event);
          if (result && typeof (result as Promise<unknown>).then === "function") {
            setPending(true);
            void Promise.resolve(result)
              .catch((error: unknown) =>
                notify({ message: (error as { detail?: string })?.detail ?? String(error) }),
              )
              .finally(() => {
                locked.current = false;
                setPending(false);
              });
          } else locked.current = false;
        } catch (error) {
          locked.current = false;
          notify({ message: (error as { detail?: string })?.detail ?? String(error) });
        }
      }}
    >
      {(pending || busy) && <Spinner size={14} />}
      {children}
    </button>
  );
});
