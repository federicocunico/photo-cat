import { useEffect, type ReactNode } from "react";

/** Avviso non modale, condiviso dalle operazioni reversibili. */
export function Toast({
  children,
  onClose,
  duration = 8000,
}: {
  children: ReactNode;
  onClose: () => void;
  duration?: number;
}) {
  useEffect(() => {
    const timer = setTimeout(onClose, duration);
    return () => clearTimeout(timer);
  }, [onClose, duration]);
  return (
    <div
      role="status"
      style={{
        position: "fixed",
        right: 24,
        bottom: 48,
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
