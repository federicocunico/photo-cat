// Impostazioni › Avanzate: valori numerici delle soglie (in bozza, si applicano con la
// barra "Applica") ed eliminazione dei dati dei volti.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, ConfirmDialog } from "../../components/ui";
import { commands, isTauri, type Taxonomy } from "../../lib/ipc";
import { fmtDec } from "../../lib/format";
import { notify, notifyError } from "../../store/toasts";
import { Setting } from "./settingsState";
import { useSettings } from "./settingsContext";
import { ThresholdInput } from "./ThresholdControls";
import { DEFAULTS, thresholdsOf, type FullThresholds } from "./tuning";

export function AdvancedSection() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { draft, setDraft, tax, setTax } = useSettings();
  const sizeId = useId();
  const [purging, setPurging] = useState(false);
  const faces = useQuery({
    queryKey: ["face-count"],
    queryFn: commands.faceCount,
    enabled: isTauri(),
  });
  const th = thresholdsOf(draft);
  const setTh = (patch: Partial<FullThresholds>) =>
    setDraft({ ...draft, ai: { ...draft.ai, thresholds: { ...th, ...patch } } });
  const setGroup = (
    gi: number,
    f: (g: Taxonomy["groups"][number]) => Taxonomy["groups"][number],
  ) => {
    if (!tax) return;
    setTax({ ...tax, groups: tax.groups.map((g, i) => (i === gi ? f(g) : g)) });
  };
  const restore = () => {
    setDraft({
      ...draft,
      ai: {
        ...draft.ai,
        face_min_size_px: DEFAULTS.face_min_size_px,
        thresholds: {
          person_detection: DEFAULTS.person_detection,
          face_detection: DEFAULTS.face_detection,
          face_min_sharpness: DEFAULTS.face_min_sharpness,
          face_similarity: DEFAULTS.face_similarity,
          similar_photos: DEFAULTS.similar_photos,
        },
      },
    });
    if (tax)
      setTax({
        ...tax,
        groups: tax.groups.map((g) => ({
          ...g,
          threshold: DEFAULTS.group,
          categories: g.categories.map((c) => ({ ...c, threshold: null })),
        })),
      });
  };
  return (
    <>
      <Setting id="rawThresholds" label={t("settings.rawThresholds")} hint={t("thresholds.hint")}>
        <div className="settings-raw">
          <label>
            <span>{t("settings.threshold")}</span>
            <ThresholdInput
              label={t("settings.threshold")}
              value={draft.ai.confidence_threshold ?? DEFAULTS.group}
              onValid={(v) =>
                setDraft({
                  ...draft,
                  ai: { ...draft.ai, confidence_threshold: v ?? DEFAULTS.group },
                })
              }
            />
          </label>
          <label>
            <span>{t("thresholds.persons")}</span>
            <ThresholdInput
              label={t("thresholds.persons")}
              value={th.person_detection}
              onValid={(v) => setTh({ person_detection: v ?? DEFAULTS.person_detection })}
            />
          </label>
          <label>
            <span>{t("thresholds.faces")}</span>
            <ThresholdInput
              label={t("thresholds.faces")}
              value={th.face_detection}
              onValid={(v) => setTh({ face_detection: v ?? DEFAULTS.face_detection })}
            />
          </label>
          <label htmlFor={sizeId}>
            <span>{t("thresholds.faceSize")}</span>
            <input
              id={sizeId}
              className="field"
              type="number"
              min={8}
              value={draft.ai.face_min_size_px ?? DEFAULTS.face_min_size_px}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  ai: { ...draft.ai, face_min_size_px: Number(e.target.value) },
                })
              }
            />
          </label>
          <label>
            <span>{t("thresholds.faceSharpness")}</span>
            <ThresholdInput
              label={t("thresholds.faceSharpness")}
              value={th.face_min_sharpness}
              onValid={(v) => setTh({ face_min_sharpness: v ?? DEFAULTS.face_min_sharpness })}
            />
          </label>
          <label>
            <span>{t("thresholds.similarity")}</span>
            <ThresholdInput
              label={t("thresholds.similarity")}
              value={th.face_similarity}
              onValid={(v) => setTh({ face_similarity: v ?? DEFAULTS.face_similarity })}
            />
          </label>
          <label title={t("similar.thresholdHint")}>
            <span>{t("similar.threshold")}</span>
            <ThresholdInput
              label={t("similar.threshold")}
              value={th.similar_photos}
              onValid={(v) => setTh({ similar_photos: v ?? DEFAULTS.similar_photos })}
            />
          </label>
        </div>
        {tax && (
          <table className="settings-table">
            <caption>{t("thresholds.groups")}</caption>
            <tbody>
              {tax.groups.map((g, gi) => [
                <tr key={g.id}>
                  <th scope="row">{g.label}</th>
                  <td>
                    <ThresholdInput
                      label={`${t("thresholds.group")} ${g.label}`}
                      value={g.threshold ?? DEFAULTS.group}
                      onValid={(v) =>
                        setGroup(gi, (x) => ({ ...x, threshold: v ?? DEFAULTS.group }))
                      }
                    />
                  </td>
                </tr>,
                ...g.categories.map((c, ci) => (
                  <tr key={`${g.id}/${c.id}`}>
                    <td className="settings-table-indent">{c.label}</td>
                    <td>
                      <ThresholdInput
                        optional
                        label={`${t("thresholds.category")} ${c.label}`}
                        placeholder={t("thresholds.inherited", {
                          value: fmtDec(g.threshold ?? DEFAULTS.group, 2),
                        })}
                        value={c.threshold ?? null}
                        onValid={(v) =>
                          setGroup(gi, (x) => ({
                            ...x,
                            categories: x.categories.map((y, j) =>
                              j === ci ? { ...y, threshold: v } : y,
                            ),
                          }))
                        }
                      />
                    </td>
                  </tr>
                )),
              ])}
            </tbody>
          </table>
        )}
        <Button variant="ghost" onClick={restore}>
          {t("thresholds.defaults")}
        </Button>
      </Setting>
      <Setting id="purgeFaces" label={t("settings.purge")} hint={t("settings.purgeHint")}>
        <Button variant="danger" onClick={() => setPurging(true)}>
          {t("settings.purge")}
        </Button>
        <ConfirmDialog
          open={purging}
          onOpenChange={setPurging}
          title={t("settings.purge")}
          message={t("settings.purgeConfirm", { count: faces.data ?? 0 })}
          confirmLabel={t("settings.purge")}
          onConfirm={async () => {
            try {
              await commands.purgeFaces();
              await qc.invalidateQueries({ queryKey: ["config"] });
              await qc.invalidateQueries({ queryKey: ["face-count"] });
              notify({ tone: "ok", message: t("settings.purged") });
            } catch (e) {
              notifyError(e);
            }
          }}
        />
      </Setting>
    </>
  );
}
