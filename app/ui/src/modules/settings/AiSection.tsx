// Impostazioni › Riconoscimento AI: attivazione, sensibilità su scala comprensibile
// (valori grezzi in Avanzate), gruppi di foto simili e tassonomia.
import { useQuery } from "@tanstack/react-query";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button, SegmentedControl, type Option } from "../../components/ui";
import { commands, isTauri, type Taxonomy } from "../../lib/ipc";
import { notify, notifyError } from "../../store/toasts";
import { Setting } from "./settingsState";
import { useSettings } from "./settingsContext";
import { SensitivitySlider } from "./ThresholdControls";
import { TaxonomySection } from "./TaxonomySection";
import { DEFAULTS, FACE_SIZES, thresholdsOf, type FullThresholds } from "./tuning";

type FaceSize = keyof typeof FACE_SIZES;

/** Interruttore salvato subito. */
function Toggle({
  id,
  label,
  checked,
  onChange,
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  const inputId = useId();
  return (
    <Setting id={id} label={label} labelFor={inputId} inline>
      <input
        id={inputId}
        type="checkbox"
        role="switch"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
    </Setting>
  );
}

export function AiSection() {
  const { t } = useTranslation();
  const { saved, draft, setDraft, tax, setTax, savedTax, autosave } = useSettings();
  const th = thresholdsOf(draft);
  const setTh = (patch: Partial<FullThresholds>) =>
    setDraft({ ...draft, ai: { ...draft.ai, thresholds: { ...th, ...patch } } });
  const setGroupThreshold = (gi: number, threshold: number) => {
    if (!tax) return;
    setTax({
      ...tax,
      groups: tax.groups.map((g, i) => (i === gi ? { ...g, threshold } : g)),
    } as Taxonomy);
  };
  // Librerie analizzate prima dei gruppi di foto simili: l'analisi si propone, non parte da sola.
  const pending = useQuery({
    queryKey: ["similar-pending"],
    queryFn: commands.similarPending,
    enabled: isTauri(),
  });
  const size = draft.ai.face_min_size_px ?? DEFAULTS.face_min_size_px;
  const sizeLevel = (Object.keys(FACE_SIZES) as FaceSize[]).find((k) => FACE_SIZES[k] === size);
  const sizes: Option<FaceSize>[] = [
    { value: "small", label: t("settings.faceSizes.small") },
    { value: "normal", label: t("settings.faceSizes.normal") },
    { value: "large", label: t("settings.faceSizes.large") },
  ];
  return (
    <>
      <Toggle
        id="faces"
        label={t("settings.faces")}
        checked={saved.ai.faces_enabled ?? true}
        onChange={(faces_enabled) =>
          autosave("faces", (c) => ({ ...c, ai: { ...c.ai, faces_enabled } }))
        }
      />
      <Toggle
        id="autoModels"
        label={t("settings.autoModels")}
        checked={saved.ai.auto_download_models ?? false}
        onChange={(auto_download_models) =>
          autosave("autoModels", (c) => ({ ...c, ai: { ...c.ai, auto_download_models } }))
        }
      />
      <Setting id="sensitivity" label={t("settings.sensitivity.title")} hint={t("thresholds.hint")}>
        <div className="settings-sliders">
          <SensitivitySlider
            scale="person_detection"
            label={t("thresholds.persons")}
            value={th.person_detection}
            onChange={(v) => setTh({ person_detection: v })}
          />
          <SensitivitySlider
            scale="face_detection"
            label={t("thresholds.faces")}
            value={th.face_detection}
            onChange={(v) => setTh({ face_detection: v })}
          />
          <SensitivitySlider
            scale="face_min_sharpness"
            label={t("thresholds.faceSharpness")}
            value={th.face_min_sharpness}
            onChange={(v) => setTh({ face_min_sharpness: v })}
          />
          <SensitivitySlider
            scale="face_similarity"
            label={t("thresholds.similarity")}
            value={th.face_similarity}
            onChange={(v) => setTh({ face_similarity: v })}
          />
          <SensitivitySlider
            scale="similar_photos"
            label={t("similar.threshold")}
            value={th.similar_photos}
            onChange={(v) => setTh({ similar_photos: v })}
          />
        </div>
      </Setting>
      <Setting id="faceSize" label={t("settings.faceSizes.label")}>
        <SegmentedControl<FaceSize>
          label={t("settings.faceSizes.label")}
          value={sizeLevel ?? ("" as FaceSize)}
          options={sizes}
          onChange={(level) =>
            setDraft({ ...draft, ai: { ...draft.ai, face_min_size_px: FACE_SIZES[level] } })
          }
        />
      </Setting>
      {(pending.data ?? 0) > 0 && (
        <Setting id="similarPending" label={t("similar.propose")}>
          <Button
            onClick={async () => {
              try {
                await commands.startSimilarAnalysis();
                notify({ tone: "ok", message: t("similar.started") });
              } catch (e) {
                notifyError(e);
              }
            }}
          >
            {t("similar.start")}
          </Button>
        </Setting>
      )}
      {tax && tax.groups.length > 0 && (
        <Setting id="groups" label={t("thresholds.groups")}>
          <div className="settings-sliders">
            {tax.groups.map((g, gi) => (
              <SensitivitySlider
                key={g.id}
                scale="group"
                label={g.label}
                value={g.threshold ?? DEFAULTS.group}
                onChange={(v) => setGroupThreshold(gi, v)}
              />
            ))}
          </div>
        </Setting>
      )}
      {tax && (
        <Setting id="taxonomy" label={t("settings.taxonomy")} hint={t("settings.taxonomyHint")}>
          <TaxonomySection
            tax={tax}
            setTax={setTax}
            saved={savedTax}
            defaultThreshold={draft.ai.confidence_threshold ?? DEFAULTS.group}
          />
        </Setting>
      )}
    </>
  );
}
