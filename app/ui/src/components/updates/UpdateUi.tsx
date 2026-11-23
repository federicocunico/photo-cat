import { useTranslation } from "react-i18next";
import { fmtInt } from "../../lib/format";
import { useUpdates } from "../../store/updates";
import { Icon } from "../icons";
import { Button, Dialog, IconButton, InlineAlert } from "../ui";
import "./updates.css";

const MB = 1024 * 1024;

/** "42% · 199 di 473 MB" dell'avanzamento del download. */
function useProgress() {
  const { t } = useTranslation();
  const done = useUpdates((s) => s.done);
  const total = useUpdates((s) => s.total);
  const percent = total > 0 ? Math.min(100, Math.floor((done / total) * 100)) : 0;
  return {
    percent,
    text: t("updates.progress", {
      percent,
      done: fmtInt(Math.round(done / MB)),
      total: fmtInt(Math.round(total / MB)),
    }),
  };
}

/**
 * Notifica di una nuova versione (app-updates): si chiude con ✕ e non ricompare fino al
 * prossimo avvio. Su Windows "Aggiorna" scarica l'installer in background con l'avanzamento;
 * su macOS e Linux apre la pagina della release.
 */
export function UpdateNotice() {
  const { t } = useTranslation();
  const s = useUpdates();
  const progress = useProgress();
  const info = s.info;
  // A download finito parla il badge nella barra superiore.
  if (!info || s.dismissed || s.phase === "idle" || s.phase === "ready") return null;
  const title = t("updates.title", { version: info.version, current: info.current });
  return (
    <section className="update-notice" role="region" aria-label={title}>
      <div className="update-notice-head">
        <Icon.Refresh size={16} aria-hidden="true" />
        <strong className="update-notice-title">{title}</strong>
        <IconButton
          label={t("updates.dismiss")}
          icon={<Icon.Close size={14} />}
          onClick={s.dismiss}
        />
      </div>
      {s.phase === "downloading" ? (
        <>
          <div
            className="bar update-bar"
            role="progressbar"
            aria-label={t("updates.downloading")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={progress.percent}
          >
            <span style={{ width: `${progress.percent}%` }} />
          </div>
          <div className="update-notice-actions">
            <span className="muted">{progress.text}</span>
            <Button size="sm" onClick={s.cancel}>
              {t("updates.stop")}
            </Button>
          </div>
        </>
      ) : (
        <>
          {s.phase === "failed" && s.error && (
            <InlineAlert tone="danger" title={t("updates.failed")}>
              {s.error}
            </InlineAlert>
          )}
          <div className="update-notice-actions">
            <Button size="sm" variant="ghost" onClick={s.openPage}>
              {t("updates.notes")}
            </Button>
            <Button size="sm" variant="primary" onClick={() => void s.download()}>
              {t(
                s.phase === "failed"
                  ? "updates.retry"
                  : info.installer
                    ? "updates.update"
                    : "updates.openPage",
              )}
            </Button>
          </div>
        </>
      )}
    </section>
  );
}

/**
 * Badge nella barra superiore: durante il download "Aggiornamento 42%" con la barra; a download
 * finito "Aggiornamento pronto", che chiede se aggiornare ora.
 */
export function UpdateBadge() {
  const { t } = useTranslation();
  const s = useUpdates();
  const progress = useProgress();
  const info = s.info;
  if (!info || (s.phase !== "downloading" && s.phase !== "ready")) return null;
  return (
    <>
      {s.phase === "downloading" ? (
        <button
          type="button"
          className="pill"
          data-testid="update-badge"
          aria-label={t("updates.badgeDownloadingLabel", { text: progress.text })}
          // Riapre la notifica con l'avanzamento e "Interrompi".
          onClick={() => useUpdates.setState({ dismissed: false })}
        >
          <Icon.Refresh size={14} aria-hidden="true" />
          <span>{t("updates.badgeDownloading", { percent: progress.percent })}</span>
          <span className="bar job-pill-bar" aria-hidden="true">
            <span style={{ width: `${progress.percent}%` }} />
          </span>
        </button>
      ) : (
        <button
          type="button"
          className="pill update-ready"
          data-testid="update-badge"
          onClick={() => s.setConfirm(true)}
        >
          <Icon.Check size={14} aria-hidden="true" />
          <span>{t("updates.badgeReady")}</span>
        </button>
      )}
      <Dialog
        open={s.confirm}
        onOpenChange={s.setConfirm}
        title={t("updates.installTitle", { version: info.version })}
        description={t("updates.installBody", { version: info.version })}
        footer={
          <>
            <Button onClick={() => s.setConfirm(false)}>{t("updates.later")}</Button>
            <Button variant="primary" onClick={() => void s.install()}>
              {t("updates.installNow")}
            </Button>
          </>
        }
      />
    </>
  );
}
