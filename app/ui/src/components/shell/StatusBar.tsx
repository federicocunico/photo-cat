import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useLive } from "../../store/live";
import { useLayout } from "../../store/layout";
import { Icon } from "../icons";
import { fmtDec, mbToGb } from "../../lib/format";
import { summarizeModels, useModelStatuses } from "../../lib/models";

export function RamChip() {
  const { t } = useTranslation();
  const res = useLive((s) => s.resources);
  const openModule = useLayout((s) => s.openModule);
  if (!res) return null;
  const { system, governor } = res;
  const used = system.totalMb - system.availableMb;
  const pressed = res.state !== "normal";
  const color = !pressed
    ? "var(--pc-ok)"
    : res.state === "pressure"
      ? "var(--pc-warn)"
      : "var(--pc-danger)";
  return (
    <button
      type="button"
      className="status-item"
      onClick={() => openModule({ module: "jobs" })}
      title={t("shell.ramTitle", {
        app: system.processMb,
        budget: governor.budgetMb,
        available: system.availableMb,
      })}
      data-testid="ram-chip"
    >
      {pressed ? (
        <Icon.Alert size={14} style={{ color }} aria-hidden="true" />
      ) : (
        <Icon.Ram size={14} style={{ color }} aria-hidden="true" />
      )}
      <span className="mono">
        {t("shell.ram", { used: mbToGb(used), total: fmtDec(system.totalMb / 1024, 0) })}
      </span>
      {/* La pressione si legge anche come testo, non solo dal colore. */}
      {pressed && <span style={{ color }}>{t(`shell.ramState.${res.state}`)}</span>}
    </button>
  );
}

/** Stato dei modelli AI: un clic apre Impostazioni › Modelli AI e sistema. */
export function ModelsChip() {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const summary = summarizeModels(useModelStatuses());
  if (summary.kind === "none") return null;
  const warn = summary.kind === "error" || summary.kind === "missing";
  return (
    <button
      type="button"
      className="status-item"
      data-testid="models-chip"
      onClick={() => openModule({ module: "settings", section: "models" })}
    >
      {warn ? (
        <Icon.Alert size={14} className="status-warn-icon" aria-hidden="true" />
      ) : (
        <Icon.Models size={14} aria-hidden="true" />
      )}
      <span>
        {summary.kind === "downloading"
          ? t("shell.models.downloading", { percent: summary.percent })
          : t(`shell.models.${summary.kind}`)}
      </span>
    </button>
  );
}

/** Barra di stato: i moduli possono mostrare conteggi propri a sinistra (`children`). */
export function StatusBar({ children }: { children?: ReactNode }) {
  const { t } = useTranslation();
  const errorCount = useLive((s) => s.errorCount);
  const openModule = useLayout((s) => s.openModule);
  return (
    <footer className="status-bar">
      {children}
      <span className="status-bar-spacer" />
      <ModelsChip />
      <RamChip />
      <button
        type="button"
        className={errorCount > 0 ? "status-item status-item-danger" : "status-item"}
        onClick={() =>
          openModule({ module: "settings", section: "diagnostics", target: { level: "ERROR" } })
        }
        data-testid="error-count"
      >
        {errorCount > 0 ? t("shell.errors", { count: errorCount }) : t("shell.noErrors")}
      </button>
    </footer>
  );
}
