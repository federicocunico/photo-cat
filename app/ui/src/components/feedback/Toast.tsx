import { useEffect, useRef, type ReactNode } from "react";

/** Avviso non modale, condiviso dalle operazioni reversibili. */
export function Toast({
  children,
  onClose,
  duration = 8000,
  stacked = false,
}: {
  children: ReactNode;
  onClose: () => void;
  duration?: number;
  stacked?: boolean;
}) {
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const timer = setTimeout(() => close.current(), duration);
    return () => clearTimeout(timer);
  }, [duration]);
  return (
    <div
      role="status"
      style={{
        position: stacked ? "relative" : "fixed",
        right: stacked ? undefined : 24,
        bottom: stacked ? undefined : 48,
        zIndex: 40,
        padding: 12,
        background: "var(--pc-raised)",
        border: "1px solid var(--pc-border)",
        borderRadius: 6,
      }}
    >
      {children}
    </div>
  );
}
