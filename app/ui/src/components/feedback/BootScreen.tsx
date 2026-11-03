import { useTranslation } from "react-i18next";
import { Spinner } from "./Spinner";
import { BusyButton } from "./BusyButton";
import type { BootStatus } from "../../lib/ipc";
import "./feedback.css";

export function BootScreen({
  status,
  retry,
  choose,
}: {
  status: BootStatus;
  retry: () => unknown;
  choose: () => unknown;
}) {
  const { t } = useTranslation();
  return (
    <main className="boot-screen" aria-busy={!status.error}>
      <h1>photo-cat</h1>
      {status.error ? (
        <>
          <h2>{t("boot.failed")}</h2>
          <p role="alert">{status.error}</p>
          <p className="mono">{t("boot.logs", { path: status.logsPath })}</p>
          <div className="feature-actions">
            <BusyButton className="btn" onClick={retry}>
              {t("feedback.retry")}
            </BusyButton>
            <BusyButton className="btn" onClick={choose}>
              {t("boot.choose")}
            </BusyButton>
          </div>
        </>
      ) : (
        <div role="status" className="boot-status">
          <Spinner />
          <span>{t(`boot.${status.phase}`, { defaultValue: t("boot.opening") })}</span>
        </div>
      )}
    </main>
  );
}
