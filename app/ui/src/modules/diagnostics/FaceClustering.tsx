// Diagnostica › "Raggruppamento dei volti" (design D8): gruppi per persona con nome, gruppi
// da un volto, unioni automatiche e annullate, soglie in uso e da dove vengono. Qui, come in
// Avanzate, i valori grezzi delle soglie sono ammessi.
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { InlineAlert } from "../../components/ui";
import { commands, isTauri } from "../../lib/ipc";
import { fmtDec, fmtInt } from "../../lib/format";

/** Soglie servono almeno 200 coppie di volti della stessa foto per essere tarate. */
const MIN_PAIRS = 200;

export function FaceClustering() {
  const { t } = useTranslation();
  const q = useQuery({
    queryKey: ["face-cluster-metrics"],
    queryFn: commands.faceClusterMetrics,
    enabled: isTauri(),
  });
  const m = q.data;
  if (!m) return null;
  const summary = [
    t("diag.faces.perNamed", { value: fmtDec(m.groupsPerNamed ?? 0, 1) }),
    t("diag.faces.singles", { value: fmtInt(Math.round((m.singleFaceShare ?? 0) * 100)) }),
    `${t("diag.faces.merges", { count: m.autoMerges })}, ${t("diag.faces.undone", { count: m.autoMergesUndone })}`,
  ].join(" · ");
  const th = m.thresholds;
  const origin = !th
    ? null
    : th.calibrated
      ? t("diag.faces.calibrated", { pairs: fmtInt(th.samePhotoPairs) })
      : th.samePhotoPairs < MIN_PAIRS
        ? t("diag.faces.defaultsFew", { pairs: fmtInt(th.samePhotoPairs), min: MIN_PAIRS })
        : t("diag.faces.defaultsKept", { pairs: fmtInt(th.samePhotoPairs) });
  return (
    <section className="ui-card diag-faces" aria-labelledby="diag-faces-title">
      <h3 id="diag-faces-title">{t("diag.faces.title")}</h3>
      <p data-testid="faces-summary">{summary}</p>
      <dl className="diag-detail-grid">
        <dt className="muted">{t("diag.faces.people")}</dt>
        <dd>
          {t("diag.faces.peopleValue", {
            named: fmtInt(m.namedPeople),
            unnamed: fmtInt(m.unnamedGroups),
          })}
        </dd>
        <dt className="muted">{t("diag.faces.queue")}</dt>
        <dd>
          {t("diag.faces.questions", { count: m.questions })} ·{" "}
          {t("diag.faces.lastRun", { count: m.lastAutoMerges })}
        </dd>
        {th && (
          <>
            <dt className="muted">{t("diag.faces.thresholds")}</dt>
            <dd data-testid="faces-thresholds">
              {t("diag.faces.thresholdsValue", {
                ask: fmtDec(th.ask ?? 0, 3),
                auto: fmtDec(th.auto ?? 0, 3),
                margin: fmtDec(th.margin ?? 0, 2),
                sensitivity: fmtDec(th.sensitivity ?? 0, 2),
              })}
            </dd>
            <dt className="muted">{t("diag.faces.origin")}</dt>
            <dd>{origin}</dd>
          </>
        )}
      </dl>
      {th?.tooStrict && <InlineAlert tone="warn">{t("diag.faces.tooStrict")}</InlineAlert>}
    </section>
  );
}
