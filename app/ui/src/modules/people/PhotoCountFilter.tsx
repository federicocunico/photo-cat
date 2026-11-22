import { useId, useState } from "react";
import { useTranslation } from "react-i18next";

const PRESETS = ["all", "1", "2", "5"] as const;

/**
 * Filtro "con al massimo N foto" per gli elenchi di persone, identità e gruppi: Tutti, un
 * valore fisso o un numero a scelta. `value` è il massimo (`null` = nessun filtro).
 */
export function PhotoCountFilter({
  value,
  onChange,
  atMost,
}: {
  value: number | null;
  onChange: (max: number | null) => void;
  /** "Fino a 1 foto" invece di "1 foto" (nelle identità c'è anche chi non ha foto). */
  atMost?: boolean;
}) {
  const { t } = useTranslation();
  const id = useId();
  const [custom, setCustom] = useState(false);
  const [text, setText] = useState("10");
  const mode = custom ? "custom" : value == null ? "all" : String(value);

  const parse = (s: string) => {
    const n = Math.floor(Number(s));
    return Number.isFinite(n) && n >= 1 ? n : null;
  };

  return (
    <div className="people-wrap people-wrap--gap-sm">
      <label htmlFor={id} className="muted people-muted-sm">
        {t("people.bulk.filterLabel")}
      </label>
      <select
        id={id}
        className="field"
        value={mode}
        onChange={(e) => {
          const v = e.target.value;
          if (v === "custom") {
            setCustom(true);
            onChange(parse(text));
          } else {
            setCustom(false);
            onChange(v === "all" ? null : Number(v));
          }
        }}
      >
        {PRESETS.map((p) => (
          <option key={p} value={p}>
            {p === "all"
              ? t("people.bulk.filterAll")
              : p === "1" && !atMost
                ? t("people.bulk.filterOne")
                : t("people.bulk.filterUpTo", { count: Number(p) })}
          </option>
        ))}
        <option value="custom">{t("people.bulk.filterCustom")}</option>
      </select>
      {custom && (
        <input
          type="number"
          min={1}
          className="field people-photo-max"
          aria-label={t("people.bulk.customLabel")}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            onChange(parse(e.target.value));
          }}
        />
      )}
    </div>
  );
}
