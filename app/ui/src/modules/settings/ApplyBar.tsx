// Barra "Ripristina / Applica" delle impostazioni che ricalcolano dati della libreria
// (soglie, tassonomia, percorso dei dati), con l'anteprima dell'effetto.
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui";
import { commands, type Taxonomy, type ThresholdImpact } from "../../lib/ipc";
import { notify } from "../../store/toasts";
import { cleanSettings } from "./cleanSettings";
import { applyFields, useSettings, withDraft } from "./settingsContext";
import { cleanTaxonomy } from "./taxonomyEdit";
import { thresholdsOf } from "./tuning";

/** Soglie di gruppi e categorie, per sapere se l'anteprima deve considerarle. */
const taxThresholds = (t: Taxonomy | null | undefined) =>
  JSON.stringify(
    t?.groups.map((g) => [g.id, g.threshold, g.categories.map((c) => [c.id, c.threshold ?? null])]),
  );

export function ApplyBar() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { saved, draft, setDraft, tax, setTax, savedTax } = useSettings();
  const [impact, setImpact] = useState<ThresholdImpact | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [shownFor, setShownFor] = useState("");

  const cfgChanged = JSON.stringify(applyFields(draft)) !== JSON.stringify(applyFields(saved));
  const taxChanged =
    tax != null && savedTax != null && JSON.stringify(tax) !== JSON.stringify(savedTax);
  const taxThresholdsChanged = taxThresholds(tax) !== taxThresholds(savedTax);
  const pathChanged = (draft.data_dir ?? null) !== (saved.data_dir ?? null);
  const withoutPath = (c: typeof saved) => JSON.stringify({ ...applyFields(c), data_dir: null });
  const thresholdsChanged = withoutPath(draft) !== withoutPath(saved);
  // L'anteprima vale solo per la bozza con cui è stata calcolata.
  const key = JSON.stringify([applyFields(draft), tax]);
  const shown = impact && shownFor === key ? impact : null;
  if (!cfgChanged && !taxChanged) return null;

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await fn();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const apply = async () => {
    if (taxChanged && tax) {
      const clean = cleanTaxonomy(tax);
      if (clean.groups.some((g) => g.categories.length === 0))
        throw { detail: t("entries.emptyGroup") };
      await commands.saveTaxonomy(clean, savedTax!.version);
    }
    if (cfgChanged) {
      const current = qc.getQueryData<typeof saved>(["config"]) ?? saved;
      await commands.saveSettings(cleanSettings(withDraft(current, draft)));
    }
    await qc.invalidateQueries({ queryKey: ["config"] });
    await qc.invalidateQueries({ queryKey: ["taxonomy"] });
    await qc.invalidateQueries({ queryKey: ["jobs"] });
  };
  const belowFloor = shown ? Math.max(shown.reanalysisFaces, shown.reanalysisPersons) : 0;

  return (
    <div className="settings-apply" role="region" aria-label={t("settings.apply.title")}>
      <div className="settings-apply-text">
        <strong>{t("settings.apply.title")}</strong>
        <span className="muted">
          {pathChanged ? t("settings.pathHint") : t("settings.apply.hint")}
        </span>
      </div>
      <div className="settings-apply-actions">
        <Button
          variant="ghost"
          disabled={busy}
          onClick={() => {
            setDraft(saved);
            if (savedTax) setTax(savedTax);
            setImpact(null);
          }}
        >
          {t("settings.apply.revert")}
        </Button>
        {(thresholdsChanged || taxThresholdsChanged) && (
          <Button
            disabled={busy}
            onClick={() =>
              run(async () => {
                setImpact(
                  await commands.previewThresholds(
                    thresholdsOf(draft),
                    taxThresholdsChanged && tax ? tax : null,
                  ),
                );
                setShownFor(key);
              })
            }
          >
            {t("thresholds.preview")}
          </Button>
        )}
        <Button
          variant="primary"
          disabled={busy}
          onClick={() =>
            run(async () => {
              await apply();
              setImpact(null);
              notify({
                tone: "ok",
                message: pathChanged ? t("settings.pathHint") : t("thresholds.applied"),
              });
            })
          }
        >
          {t("settings.apply.apply")}
        </Button>
      </div>
      {busy && !shown && <span className="muted">{t("thresholds.previewing")}</span>}
      {shown && (
        <ul role="status" aria-label={t("thresholds.preview")} className="settings-impact">
          <li>{t("thresholds.labelsChanged", { count: shown.mediaLabelsChanged })}</li>
          <li>{t("thresholds.peopleChanged", { count: shown.mediaPeopleChanged })}</li>
          <li>{t("thresholds.facesExcluded", { count: shown.facesExcluded })}</li>
          <li>
            {t("thresholds.facesDiscardedSharpness", { count: shown.facesDiscardedSharpness })}
          </li>
          {shown.personsAffectedEstimate > 0 && (
            <li>{t("thresholds.personsEstimate", { count: shown.personsAffectedEstimate })}</li>
          )}
          <li data-testid="similar-preview">
            {t("similar.preview", { n: shown.similarGroups, current: shown.similarGroupsCurrent })}
          </li>
        </ul>
      )}
      {belowFloor > 0 && (
        <div className="settings-apply-warn">
          {t("thresholds.belowFloor", { count: belowFloor })}{" "}
          <Button
            disabled={busy}
            onClick={() =>
              run(async () => {
                await apply();
                await commands.reanalyzeBelowFloor();
                setImpact(null);
                notify({ tone: "ok", message: t("thresholds.reanalyzeStarted") });
              })
            }
          >
            {t("thresholds.reanalyze")}
          </Button>
        </div>
      )}
      {error && (
        <p role="alert" className="settings-save-error">
          {error}
        </p>
      )}
    </div>
  );
}
