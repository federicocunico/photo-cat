import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ModuleNav } from "./ModuleNav";
import { TopBar } from "./TopBar";
import { useLayout } from "../../store/layout";
import type { CrashInfo } from "../../lib/ipc";

export function CrashBanner({ crash, onDismiss }: { crash: CrashInfo; onDismiss: () => void }) {
  const { t } = useTranslation();
  const openDiagnostics = useLayout((s) => s.openDiagnostics);
  return (
    <div
      role="alert"
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: 12,
        padding: "8px 16px",
        background: "var(--pc-selection-bg)",
        borderBottom: "1px solid var(--pc-selection-border)",
      }}
    >
      <strong>{t("shell.crashNotice")}</strong>
      <span className="mono muted" style={{ fontSize: 12 }}>
        {crash.message}
      </span>
      <button
        type="button"
        className="btn"
        style={{ marginLeft: "auto" }}
        onClick={() => openDiagnostics({})}
      >
        {t("shell.crashOpen")}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onDismiss}>
        {t("shell.dismiss")}
      </button>
    </div>
  );
}

/** Struttura comune: barra superiore, navigazione moduli, contenuto del modulo. */
export function AppShell({ children, banner }: { children: ReactNode; banner?: ReactNode }) {
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <TopBar />
      {banner}
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <ModuleNav />
        <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
          {children}
        </main>
      </div>
    </div>
  );
}
