import { useId } from "react";
import { useTranslation } from "react-i18next";
import { useLayout } from "../../store/layout";
import { useUpdates } from "../../store/updates";
import { Button } from "../ui";
import "./updates.css";

/**
 * Impostazioni › Informazioni (app-about, "Aggiornamenti nelle Informazioni"): controllo
 * all'avvio e "Controlla ora", con l'esito in parole.
 */
export function UpdateSettings() {
  const { t } = useTranslation();
  const titleId = useId();
  const onStart = useLayout((s) => s.checkUpdatesOnStart);
  const setOnStart = useLayout((s) => s.setCheckUpdatesOnStart);
  const checking = useUpdates((s) => s.checking);
  const lastCheck = useUpdates((s) => s.lastCheck);
  return (
    <section className="feature-card update-settings" aria-labelledby={titleId}>
      <h2 id={titleId} className="section-title">
        {t("updates.settingsTitle")}
      </h2>
      <label className="update-settings-check">
        <input type="checkbox" checked={onStart} onChange={(e) => setOnStart(e.target.checked)} />
        {t("updates.checkOnStart")}
      </label>
      <div className="update-settings-row">
        <Button disabled={checking} onClick={() => void useUpdates.getState().check(true)}>
          {t("updates.checkNow")}
        </Button>
        <span role="status" className="muted">
          {checking ? t("updates.checking") : lastCheck}
        </span>
      </div>
    </section>
  );
}
