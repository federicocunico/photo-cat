// Verifica della fotocamera (porting di CameraDiagnostics di FujiView): stato, esito di ogni
// controllo con il suggerimento, prima configurazione e registro recente.
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { Button } from "../../components/ui";
import type { CheckStatus } from "../../lib/ipc";
import { fmtTime } from "../../lib/format";
import { headline, logToText } from "./develop";
import type { CameraMonitor } from "./useCamera";

const STATUS_ICON: Record<CheckStatus, ReactNode> = {
  ok: <Icon.Check size={16} />,
  warning: <Icon.Alert size={16} />,
  error: <Icon.Close size={16} />,
  pending: <Icon.Spinner size={16} />,
  skipped: <Icon.Minus size={16} />,
  info: <Icon.Info size={16} />,
};

/** Prima configurazione della fotocamera: come nel README di FujiView. */
export function CameraSetupGuide({ open }: { open?: boolean }) {
  const { t } = useTranslation();
  return (
    <details className="cc-guide" open={open}>
      <summary>{t("rawCamera.guide.title")}</summary>
      <ol>
        <li>{t("rawCamera.guide.connect")}</li>
        <li>{t("rawCamera.guide.usbMode")}</li>
        <li>{t("rawCamera.guide.autoPowerOff")}</li>
        <li>{t("rawCamera.guide.closePrograms")}</li>
        <li>{t("rawCamera.guide.sameCamera")}</li>
        <li>{t("rawCamera.guide.frozen")}</li>
      </ol>
    </details>
  );
}

/** Registro recente della fotocamera, espandibile e copiabile per l'assistenza. */
export function CameraLog({ monitor }: { monitor: CameraMonitor }) {
  const { t } = useTranslation();
  const { log } = monitor;
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(logToText(log));
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };
  return (
    <details className="cc-log">
      <summary>{t("rawCamera.log.title", { count: log.length })}</summary>
      <div className="cc-log-actions">
        <Button size="sm" icon={<Icon.Copy size={14} />} disabled={log.length === 0} onClick={copy}>
          {copied ? t("rawCamera.log.copied") : t("rawCamera.log.copy")}
        </Button>
      </div>
      <pre className="cc-log-text" tabIndex={0} aria-label={t("rawCamera.log.label")}>
        {log.length === 0
          ? t("rawCamera.log.empty")
          : log.map((l, i) => (
              <span key={i} className={`cc-log-${l.level}`}>
                {fmtTime(l.at)} {l.message}
                {"\n"}
              </span>
            ))}
      </pre>
    </details>
  );
}

export function CameraCheck({
  monitor,
  compact = false,
}: {
  monitor: CameraMonitor;
  /** Controlli chiusi finché non servono (durante una pausa). */
  compact?: boolean;
}) {
  const { t } = useTranslation();
  const { status, diagnostics, checking, recheck } = monitor;
  const [showChecks, setShowChecks] = useState(!compact);
  const state = status?.state;
  const tone = headline(status, diagnostics, checking);
  const hasProblem = !!diagnostics && !diagnostics.ready && !diagnostics.running;
  const noCamera = state === "disconnected" || state === "notReady";

  return (
    <div className={`cc-panel${compact ? " cc-compact" : ""}`}>
      <div className="cc-status">
        <span className={`cc-dot cc-${state ?? "unknown"}`} aria-hidden="true" />
        <span>
          <strong>{status?.model ?? t("rawCamera.generic")}</strong>
          {" · "}
          {t(`rawCamera.state.${state ?? "unknown"}`)}
          {status?.firmware ? ` · ${t("rawCamera.firmware", { version: status.firmware })}` : ""}
          {status?.simulated ? ` · ${t("rawCamera.simulated")}` : ""}
        </span>
        <Button
          size="sm"
          icon={checking ? <Icon.Spinner size={14} /> : <Icon.Refresh size={14} />}
          disabled={checking}
          onClick={() => void recheck()}
        >
          {checking ? t("rawCamera.rechecking") : t("rawCamera.recheck")}
        </Button>
      </div>
      <p
        className={`cc-headline cc-tone-${tone}`}
        role="status"
        aria-busy={tone === "checking" || undefined}
      >
        {t(`rawCamera.headline.${tone}`)}
      </p>
      {diagnostics && (
        <>
          <button
            type="button"
            className="cc-toggle"
            aria-expanded={showChecks}
            onClick={() => setShowChecks((v) => !v)}
          >
            {showChecks ? <Icon.ChevronDown size={14} /> : <Icon.ChevronRight size={14} />}
            {diagnostics.checkedAt
              ? t("rawCamera.checksAt", { time: fmtTime(diagnostics.checkedAt) })
              : t("rawCamera.checks")}
          </button>
          {showChecks && (
            <ul className="cc-checks" aria-label={t("rawCamera.checks")}>
              {diagnostics.checks.map((c) => (
                <li key={c.id} className={`cc-check cc-check-${c.status}`}>
                  <span className="cc-check-icon" aria-hidden="true">
                    {STATUS_ICON[c.status]}
                  </span>
                  <span className="cc-check-body">
                    <strong>
                      <span className="visually-hidden">
                        {t(`rawCamera.checkStatus.${c.status}`)}:{" "}
                      </span>
                      {t(`rawCamera.checkLabels.${c.id}`, { defaultValue: c.label })}
                    </strong>
                    {c.detail && <small>{c.detail}</small>}
                    {c.hint &&
                      c.status !== "ok" &&
                      c.status !== "pending" &&
                      c.status !== "skipped" && <em>{c.hint}</em>}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      <CameraSetupGuide open={noCamera || hasProblem} />
      <CameraLog monitor={monitor} />
    </div>
  );
}
