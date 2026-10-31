import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useLive } from "../../store/live";
import { useLayout } from "../../store/layout";

/** Barra di stato: i moduli possono mostrare conteggi propri a sinistra (`children`). */
export function StatusBar({ children }: { children?: ReactNode }) {
  const { t } = useTranslation();
  const errorCount = useLive((s) => s.errorCount);
  const openDiagnostics = useLayout((s) => s.openDiagnostics);
  return (
    <footer
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: 20,
        alignItems: "center",
        padding: "6px 16px",
        background: "var(--pc-panel)",
        borderTop: "1px solid var(--pc-border)",
        fontSize: 12,
        color: "var(--pc-text-secondary)",
      }}
    >
      {children}
      <button
        type="button"
        onClick={() => openDiagnostics({ level: "ERROR" })}
        data-testid="error-count"
        style={{
          marginLeft: "auto",
          background: "transparent",
          border: 0,
          cursor: "pointer",
          color: errorCount > 0 ? "var(--pc-danger)" : "var(--pc-text-secondary)",
          fontSize: 12,
        }}
      >
        {errorCount > 0 ? t("shell.errors", { count: errorCount }) : t("shell.noErrors")}
      </button>
    </footer>
  );
}
