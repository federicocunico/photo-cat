import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ModuleNav } from "./ModuleNav";
import { TopBar } from "./TopBar";
import { useLayout } from "../../store/layout";
import { useUndoHotkey } from "../../store/undo";
import { fmtDateTime } from "../../lib/format";
import { CommandPalette } from "../command/CommandPalette";
import { usePaletteCommand } from "../../commands/paletteCommand";
import { ShortcutSheet } from "../command/ShortcutSheet";
import { GuideDialog } from "../guide/GuideDialog";
import { useShortcutCatalog } from "../../commands/catalog";
import { useSheetCommand } from "../../commands/sheetCommand";
import type { CrashInfo, UiRecovery } from "../../lib/ipc";
import "./shell.css";

const time = (iso: string) => fmtDateTime(iso);

/** Avviso di chiusura anomala della sessione precedente. */
export function CrashBanner({ crash, onDismiss }: { crash: CrashInfo; onDismiss: () => void }) {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const kind = crash.kind ?? "panic";
  return (
    <div role="alert" className="shell-banner">
      <strong>{t("shell.crashNotice")}</strong>
      <span className="muted shell-banner-detail">
        {t(`shell.crashKind.${kind}`)}
        {crash.lastActivity && ` · ${t("shell.crashLast", { time: time(crash.lastActivity) })}`}
        {crash.memory &&
          ` · ${t("shell.crashMemory", {
            app: crash.memory.appMb,
            ui: crash.memory.webviewMb,
            free: crash.memory.availableMb,
          })}`}
      </span>
      {crash.message && <span className="mono muted shell-banner-detail">{crash.message}</span>}
      <button
        type="button"
        className="btn shell-banner-action"
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
    <div role="alert" className="shell-banner" data-testid="ui-recovered">
      <strong>{t("shell.uiRecovered")}</strong>
      <span className="muted shell-banner-detail">
        {t("shell.uiRecoveredDetail", { time: time(info.ts) })}
      </span>
      <span className="mono muted shell-banner-detail">{info.reason}</span>
      <button
        type="button"
        className="btn shell-banner-action"
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
  usePaletteCommand();
  useShortcutCatalog();
  useSheetCommand();
  return (
    <div className="app-shell">
      <TopBar />
      {banner}
      <div className="app-shell-body">
        <ModuleNav />
        <main className="app-shell-main">{children}</main>
        <CommandPalette />
        <ShortcutSheet />
        <GuideDialog />
      </div>
    </div>
  );
}
