import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ModuleNav } from "./ModuleNav";
import { TopBar } from "./TopBar";
import { useLayout } from "../../store/layout";
import { useUndoHotkey } from "../../store/undo";
import { fmtDateTime } from "../../lib/format";
import type { CrashInfo, UiRecovery } from "../../lib/ipc";

const bannerStyle = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: 12,
  padding: "8px 16px",
  background: "var(--pc-selection-bg)",
  borderBottom: "1px solid var(--pc-selection-border)",
} as const;

const time = (iso: string) => fmtDateTime(iso);

/** Avviso di chiusura anomala della sessione precedente. */
export function CrashBanner({ crash, onDismiss }: { crash: CrashInfo; onDismiss: () => void }) {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const kind = crash.kind ?? "panic";
  return (
    <div role="alert" style={bannerStyle}>
      <strong>{t("shell.crashNotice")}</strong>
      <span className="muted" style={{ fontSize: 12 }}>
        {t(`shell.crashKind.${kind}`)}
        {crash.lastActivity && ` · ${t("shell.crashLast", { time: time(crash.lastActivity) })}`}
        {crash.memory &&
          ` · ${t("shell.crashMemory", {
            app: crash.memory.appMb,
            ui: crash.memory.webviewMb,
            free: crash.memory.availableMb,
          })}`}
      </span>
      {crash.message && (
        <span className="mono muted" style={{ fontSize: 12 }}>
          {crash.message}
        </span>
      )}
      <button
        type="button"
        className="btn"
        style={{ marginLeft: "auto" }}
        onClick={() => openModule({ module: "settings", section: "diagnostics", target: {} })}
      >
        {t("shell.crashOpen")}
      </button>
      <button type="button" className="btn btn-ghost" onClick={onDismiss}>
        {t("shell.dismiss")}
      </button>
    </div>
  );
}

/** L'interfaccia è stata ricaricata dopo la terminazione della WebView (i job continuano). */
export function UiRecoveredBanner({
  info,
  onDismiss,
}: {
  info: UiRecovery;
  onDismiss: () => void;
}) {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  return (
    <div role="alert" style={bannerStyle} data-testid="ui-recovered">
      <strong>{t("shell.uiRecovered")}</strong>
      <span className="muted" style={{ fontSize: 12 }}>
        {t("shell.uiRecoveredDetail", { time: time(info.ts) })}
      </span>
      <span className="mono muted" style={{ fontSize: 12 }}>
        {info.reason}
      </span>
      <button
        type="button"
        className="btn"
        style={{ marginLeft: "auto" }}
        onClick={() => openModule({ module: "settings", section: "diagnostics", target: {} })}
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
  useUndoHotkey();
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
