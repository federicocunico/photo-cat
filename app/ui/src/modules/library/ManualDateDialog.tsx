// Dialogo "Imposta data di scatto": data a mano con precisione giorno, mese o anno.
// Riusabile dal menu della griglia e dall'Inspector.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { DatePrecision } from "../../bindings";
import { Button, Dialog, FormField, SegmentedControl } from "../../components/ui";

const pad = (n: number, w = 2) => String(n).padStart(w, "0");

/** Valore "AAAA", "AAAA-MM" o "AAAA-MM-GG", o `null` se la data non esiste. */
export function manualDateValue(
  precision: DatePrecision,
  year: number,
  month: number,
  day: number,
): string | null {
  if (!Number.isInteger(year) || year < 1 || year > 9999) return null;
  if (precision === "year") return pad(year, 4);
  if (!Number.isInteger(month) || month < 1 || month > 12) return null;
  if (precision === "month") return `${pad(year, 4)}-${pad(month)}`;
  const d = new Date(Date.UTC(2000, month - 1, day));
  d.setUTCFullYear(year);
  if (
    !Number.isInteger(day) ||
    d.getUTCFullYear() !== year ||
    d.getUTCMonth() !== month - 1 ||
    d.getUTCDate() !== day
  )
    return null;
  return `${pad(year, 4)}-${pad(month)}-${pad(day)}`;
}

export function ManualDateDialog({
  open,
  onOpenChange,
  count,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  count: number;
  onConfirm: (value: string, precision: DatePrecision) => void;
}) {
  const { t } = useTranslation();
  const now = new Date();
  const [precision, setPrecision] = useState<DatePrecision>("day");
  const [year, setYear] = useState(String(now.getFullYear()));
  const [month, setMonth] = useState(String(now.getMonth() + 1));
  const [day, setDay] = useState(String(now.getDate()));
  const value = manualDateValue(precision, Number(year), Number(month), Number(day));
  const submit = () => {
    if (!value) return;
    onConfirm(value, precision);
    onOpenChange(false);
  };
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("selection.dateTitle", { count })}
      width={380}
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>
          <Button variant="primary" disabled={!value} onClick={submit}>
            {t("selection.dateApply")}
          </Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <FormField label={t("library.inspector.datePrecision")}>
          <SegmentedControl
            label={t("library.inspector.datePrecision")}
            value={precision}
            onChange={setPrecision}
            options={(["day", "month", "year"] as const).map((p) => ({
              value: p,
              label: t(`library.inspector.precision.${p}`),
            }))}
          />
        </FormField>
        <FormField label={t("library.inspector.dateYear")}>
          <input
            className="field"
            type="number"
            min={1}
            max={9999}
            autoFocus
            value={year}
            onChange={(e) => setYear(e.target.value)}
          />
        </FormField>
        {precision !== "year" && (
          <FormField label={t("library.inspector.dateMonth")}>
            <input
              className="field"
              type="number"
              min={1}
              max={12}
              value={month}
              onChange={(e) => setMonth(e.target.value)}
            />
          </FormField>
        )}
        {precision === "day" && (
          <FormField label={t("library.inspector.dateDay")}>
            <input
              className="field"
              type="number"
              min={1}
              max={31}
              value={day}
              onChange={(e) => setDay(e.target.value)}
            />
          </FormField>
        )}
        {!value && <span className="muted">{t("selection.dateInvalid")}</span>}
      </form>
    </Dialog>
  );
}
