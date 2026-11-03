// Controlli delle soglie: la scala "Più risultati ↔ Più precisi" (Riconoscimento AI) e
// il valore numerico 0–1 (Avanzate).
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { fmtDec } from "../../lib/format";
import { fromPrecision, precision, SCALE } from "./tuning";

/**
 * Campo numerico 0–1: un valore non valido viene mostrato come errore e non
 * sostituisce quello in vigore.
 */
export function ThresholdInput({
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
    <span className="settings-threshold">
      <input
        className="field"
        type="text"
        inputMode="decimal"
        aria-label={label}
        aria-invalid={invalid}
        placeholder={placeholder}
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
        <span role="alert" className="settings-save-error">
          {t("thresholds.invalid", { value: value == null ? "—" : fmtDec(value, 3) })}
        </span>
      )}
    </span>
  );
}

/**
 * Sensibilità di una soglia su una scala comprensibile: a sinistra "Più risultati", a
 * destra "Più precisi", qualunque sia il verso del valore grezzo (tabella `SCALE`).
 */
export function SensitivitySlider({
  scale,
  label,
  value,
  onChange,
}: {
  scale: keyof typeof SCALE;
  label: string;
  value: number;
  onChange: (v: number) => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const { lo, hi } = SCALE[scale];
  const pos = precision(scale, value);
  const min = Math.min(precision(scale, lo), precision(scale, hi));
  const max = Math.max(precision(scale, lo), precision(scale, hi));
  return (
    <div className="settings-slider">
      <label htmlFor={id} className="settings-slider-label">
        {label}
      </label>
      <div className="settings-slider-row">
        <span className="muted" aria-hidden="true">
          {t("settings.sensitivity.more")}
        </span>
        <input
          id={id}
          type="range"
          min={min}
          max={max}
          step={1}
          value={Math.min(max, Math.max(min, pos))}
          aria-valuetext={t("settings.sensitivity.value", { percent: pos })}
          onChange={(e) => onChange(fromPrecision(scale, Number(e.target.value)))}
        />
        <span className="muted" aria-hidden="true">
          {t("settings.sensitivity.precise")}
        </span>
      </div>
    </div>
  );
}
