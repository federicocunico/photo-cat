import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { IconButton } from "../../components/ui";
import { commands, isTauri } from "../../lib/ipc";
import { summarizeModels } from "../../lib/models";
import { useLayout, type Destination } from "../../store/layout";
import { doneCount, GUIDE_STEPS, useGuide, type GuideStep } from "../../store/guide";
import "../../components/guide/guide.css";

/** Dove porta ogni voce della lista. */
const DESTINATION: Record<GuideStep, Destination> = {
  import: { module: "import" },
  models: { module: "settings", section: "models" },
  person: { module: "people" },
  catalog: { module: "catalogs" },
  tags: { module: "tags" },
};

/**
 * Completa le voci osservando i dati che le altre parti dell'app già aggiornano: una
 * voce risulta fatta anche se l'utente ci arriva per un'altra strada (es. un catalogo
 * creato dalla barra delle azioni invalida `["catalogs"]`). L'uso di un tasto di un set
 * di tag è segnato da `useTagHotkeys`.
 */
function useStepDetection(enabled: boolean) {
  const done = useGuide((s) => s.done);
  const complete = useGuide((s) => s.complete);
  const on = (step: GuideStep) => enabled && isTauri() && !done[step];
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: () => commands.listSources(),
    enabled: on("import"),
  });
  const models = useQuery({
    queryKey: ["models"],
    queryFn: () => commands.listModels(),
    enabled: on("models"),
  });
  const sections = useQuery({
    queryKey: ["people-sections"],
    queryFn: () => commands.peopleSections(),
    enabled: on("person"),
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: on("catalog"),
  });
  useEffect(() => {
    if (sources.data?.length) complete("import");
    if (models.data && summarizeModels(models.data).kind === "ready") complete("models");
    if ((sections.data?.people ?? 0) > 0) complete("person");
    if (catalogs.data?.length) complete("catalog");
  }, [sources.data, models.data, sections.data, catalogs.data, complete]);
}

/** Lista "Primi passi" in cima alla Libreria (in-app-guidance). */
export function FirstSteps() {
  const { t } = useTranslation();
  const done = useGuide((s) => s.done);
  const hidden = useGuide((s) => s.hidden);
  const hide = useGuide((s) => s.hide);
  const openModule = useLayout((s) => s.openModule);
  const count = doneCount(done);
  const visible = !hidden && count < GUIDE_STEPS.length;
  useStepDetection(visible);
  if (!visible) return null;
  const total = GUIDE_STEPS.length;
  return (
    <section className="first-steps" aria-labelledby="first-steps-title">
      <h2 id="first-steps-title" className="first-steps-title">
        {t("guide.firstSteps")}
        <span className="muted"> · {t("guide.progress", { done: count, total })}</span>
      </h2>
      <span
        className="bar first-steps-bar"
        role="progressbar"
        aria-labelledby="first-steps-title"
        aria-valuemin={0}
        aria-valuemax={total}
        aria-valuenow={count}
      >
        <span style={{ width: `${(count / total) * 100}%` }} />
      </span>
      <ul className="first-steps-list">
        {GUIDE_STEPS.map((step) => {
          const name = t(`guide.steps.${step}`);
          return (
            <li key={step}>
              {done[step] ? (
                <span className="first-steps-done">
                  <Icon.Check size={14} aria-hidden="true" />
                  <span aria-hidden="true">{name}</span>
                  <span className="visually-hidden">{t("guide.stepDone", { name })}</span>
                </span>
              ) : (
                <button
                  type="button"
                  className="link-button"
                  onClick={() => openModule(DESTINATION[step])}
                >
                  {name}
                </button>
              )}
            </li>
          );
        })}
      </ul>
      <IconButton
        label={t("guide.hide")}
        icon={<Icon.Close size={16} />}
        variant="ghost"
        className="first-steps-hide"
        onClick={hide}
      />
    </section>
  );
}
