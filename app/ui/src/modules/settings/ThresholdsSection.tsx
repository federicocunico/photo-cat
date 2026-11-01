// Sezione "Soglie AI": soglie dei modelli (configurazione) e di gruppi e
// categorie CLIP (tassonomia), con anteprima dell'effetto calcolata dai
// risultati salvati e applicazione senza rianalisi.
import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  type AiThresholds,
  type Settings,
  type Taxonomy,
  type ThresholdImpact,
} from "../../lib/ipc";
import { fmtDec } from "../../lib/format";
import { cleanSettings } from "./cleanSettings";

/** Valori predefiniti dell'applicazione (pc-core `AiThresholds`, tassonomia). */
const DEFAULTS = {
  person_detection: 0.4,
  face_detection: 0.8,
  face_similarity: 0.363,
  face_min_size_px: 40,
  group: 0.25,
};

type Full = Required<{ [K in keyof AiThresholds]: number }>;

function thresholdsOf(cfg: Settings): Full {
  const t = cfg.ai.thresholds ?? {};
  return {
    person_detection: t.person_detection ?? DEFAULTS.person_detection,
    face_detection: t.face_detection ?? DEFAULTS.face_detection,
    face_similarity: t.face_similarity ?? DEFAULTS.face_similarity,
  };
}

/** Soglie di gruppi e categorie, per capire se la tassonomia è cambiata. */
const taxThresholds = (t: Taxonomy | null | undefined) =>
  JSON.stringify(
    t?.groups.map((g) => [g.id, g.threshold, g.categories.map((c) => [c.id, c.threshold ?? null])]),
  );

/**
 * Campo numerico 0–1: un valore non valido viene mostrato come errore e non
 * sostituisce quello in vigore.
 */
function ThresholdInput({
  label,
  value,
  onValid,
  optional,
  placeholder,
}: {
  label: string;
  value: number | null;
  onValid: (v: number | null) => void;
  optional?: boolean;
  placeholder?: string;
}) {
  const { t } = useTranslation();
  const [text, setText] = useState(value == null ? "" : String(value));
  const [invalid, setInvalid] = useState(false);
  useEffect(() => {
    setText(value == null ? "" : String(value));
    setInvalid(false);
  }, [value]);
  return (
    <span style={{ display: "inline-flex", flexDirection: "column", gap: 2 }}>
      <input
        className="field"
        type="text"
        inputMode="decimal"
        aria-label={label}
        aria-invalid={invalid}
        placeholder={placeholder}
        style={{ width: 90 }}
        value={text}
        onChange={(e) => {
          const raw = e.target.value;
          setText(raw);
          if (optional && raw.trim() === "") {
            setInvalid(false);
            onValid(null);
            return;
          }
          const n = Number(raw.replace(",", "."));
          const ok = raw.trim() !== "" && Number.isFinite(n) && n >= 0 && n <= 1;
          setInvalid(!ok);
          if (ok) onValid(n);
        }}
      />
      {invalid && (
        <span role="alert" style={{ fontSize: 12, color: "var(--pc-warn)" }}>
          {t("thresholds.invalid", { value: value == null ? "—" : fmtDec(value, 3) })}
        </span>
      )}
    </span>
  );
}

export function ThresholdsSection({
  cfg,
  setCfg,
  tax,
  setTax,
  saved,
  savedTax,
}: {
  cfg: Settings;
  setCfg: (c: Settings) => void;
  tax: Taxonomy | null;
  setTax: (t: Taxonomy) => void;
  /** Configurazione salvata (per sapere cosa è cambiato). */
  saved: Settings;
  savedTax: Taxonomy | undefined;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const th = thresholdsOf(cfg);
  const [impact, setImpact] = useState<ThresholdImpact | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const taxChanged = taxThresholds(tax) !== taxThresholds(savedTax);
  const cfgChanged =
    JSON.stringify(th) !== JSON.stringify(thresholdsOf(saved)) ||
    cfg.ai.face_min_size_px !== saved.ai.face_min_size_px;

  const setTh = (patch: Partial<Full>) => {
    setImpact(null);
    setCfg({ ...cfg, ai: { ...cfg.ai, thresholds: { ...th, ...patch } } });
  };
  const setGroup = (
    gi: number,
    f: (g: Taxonomy["groups"][number]) => Taxonomy["groups"][number],
  ) => {
    if (!tax) return;
    setImpact(null);
    setTax({ ...tax, groups: tax.groups.map((g, i) => (i === gi ? f(g) : g)) });
  };
  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await fn();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const apply = async () => {
    if (cfgChanged) await commands.saveSettings(cleanSettings(cfg));
    if (taxChanged && tax && savedTax) await commands.saveTaxonomy(tax, savedTax.version);
    await qc.invalidateQueries({ queryKey: ["config"] });
    await qc.invalidateQueries({ queryKey: ["taxonomy"] });
    await qc.invalidateQueries({ queryKey: ["jobs"] });
  };
  const restore = () => {
    setImpact(null);
    setCfg({
      ...cfg,
      ai: {
        ...cfg.ai,
        face_min_size_px: DEFAULTS.face_min_size_px,
        thresholds: {
          person_detection: DEFAULTS.person_detection,
          face_detection: DEFAULTS.face_detection,
          face_similarity: DEFAULTS.face_similarity,
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
  const belowFloor = impact ? Math.max(impact.reanalysisFaces, impact.reanalysisPersons) : 0;

  return (
    <section aria-labelledby="thresholds-title" style={{ margin: "24px 0" }}>
      <h2 id="thresholds-title">{t("thresholds.title")}</h2>
      <p className="muted">{t("thresholds.hint")}</p>
      <div className="feature-card" style={{ alignItems: "stretch", gap: 10 }}>
        <label>
          {t("thresholds.persons")}{" "}
          <ThresholdInput
            label={t("thresholds.persons")}
            value={th.person_detection}
            onValid={(v) => setTh({ person_detection: v ?? DEFAULTS.person_detection })}
          />
        </label>
        <label>
          {t("thresholds.faces")}{" "}
          <ThresholdInput
            label={t("thresholds.faces")}
            value={th.face_detection}
            onValid={(v) => setTh({ face_detection: v ?? DEFAULTS.face_detection })}
          />
        </label>
        <label>
          {t("thresholds.faceSize")}{" "}
          <input
            className="field"
            type="number"
            min={8}
            style={{ width: 90 }}
            value={cfg.ai.face_min_size_px}
            onChange={(e) =>
              setCfg({ ...cfg, ai: { ...cfg.ai, face_min_size_px: Number(e.target.value) } })
            }
          />
        </label>
        <label>
          {t("thresholds.similarity")}{" "}
          <ThresholdInput
            label={t("thresholds.similarity")}
            value={th.face_similarity}
            onValid={(v) => setTh({ face_similarity: v ?? DEFAULTS.face_similarity })}
          />
        </label>
        {tax && (
          <table style={{ borderCollapse: "collapse", fontSize: 13 }}>
            <caption style={{ textAlign: "left", fontWeight: 600, padding: "6px 0" }}>
              {t("thresholds.groups")}
            </caption>
            <tbody>
              {tax.groups.map((g, gi) => [
                <tr key={g.id}>
                  <th scope="row" style={{ textAlign: "left", padding: "4px 8px 4px 0" }}>
                    {g.label}
                  </th>
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
                    <td style={{ padding: "2px 8px 2px 16px" }}>{c.label}</td>
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
        <div className="feature-actions" style={{ margin: 0 }}>
          <button
            className="btn"
            disabled={busy}
            onClick={() =>
              void run(async () => {
                setImpact(await commands.previewThresholds(th, taxChanged && tax ? tax : null));
              })
            }
          >
            {t("thresholds.preview")}
          </button>
          <button
            className="btn"
            disabled={busy || (!cfgChanged && !taxChanged)}
            onClick={() =>
              void run(async () => {
                await apply();
                setNotice(t("thresholds.applied"));
              })
            }
          >
            {t("thresholds.apply")}
          </button>
          <button className="btn btn-ghost" disabled={busy} onClick={restore}>
            {t("thresholds.defaults")}
          </button>
        </div>
        {busy && !impact && <span className="muted">{t("thresholds.previewing")}</span>}
        {impact && (
          <ul role="status" aria-label={t("thresholds.preview")} style={{ margin: 0 }}>
            <li>{t("thresholds.labelsChanged", { count: impact.mediaLabelsChanged })}</li>
            <li>{t("thresholds.peopleChanged", { count: impact.mediaPeopleChanged })}</li>
            <li>{t("thresholds.facesExcluded", { count: impact.facesExcluded })}</li>
            {impact.personsAffectedEstimate > 0 && (
              <li>{t("thresholds.personsEstimate", { count: impact.personsAffectedEstimate })}</li>
            )}
          </ul>
        )}
        {belowFloor > 0 && (
          <div style={{ color: "var(--pc-warn)" }}>
            {t("thresholds.belowFloor", { count: belowFloor })}{" "}
            <button
              className="btn"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  await apply();
                  await commands.reanalyzeBelowFloor();
                  setNotice(t("thresholds.reanalyzeStarted"));
                })
              }
            >
              {t("thresholds.reanalyze")}
            </button>
          </div>
        )}
        {error && <p role="alert">{error}</p>}
        {notice && <p role="status">{notice}</p>}
      </div>
    </section>
  );
}
