// Valutazione a stelle, caratteristica di colore e data di scatto dell'Inspector
// (github-issues-39-47 2.1). Le modifiche passano dal bulk con "Annulla operazione".
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, InlineEdit, SegmentedControl } from "../../components/ui";
import type { BulkAction, ColorTrait, DatePrecision, MediaDates } from "../../lib/ipc";
import { fmtPeriod } from "../../lib/format";
import i18n from "../../i18n";

const COLOR_TRAITS: ColorTrait[] = ["bw", "color", "faded", "sepia"];

type Apply = (operation: BulkAction) => Promise<unknown>;

/** Stelle cliccabili 0–5: cliccare la stella corrente (o "Nessuna") azzera. */
export function RatingRow({
  apply,
  rating,
}: {
  apply: Apply;
  /** `undefined`: valutazioni diverse tra le foto selezionate. */
  rating: number | undefined;
}) {
  const { t } = useTranslation();
  return (
    <>
      <span className="muted">{t("library.inspector.rating")}</span>
      <div
        className="inspector-rating"
        role="group"
        aria-label={t("library.inspector.rating")}
        data-testid="inspector-rating"
      >
        {[1, 2, 3, 4, 5].map((n) => (
          <Button
            key={n}
            size="sm"
            variant="ghost"
            aria-label={t("library.inspector.stars", { count: n })}
            aria-pressed={rating === n}
            onClick={() => void apply({ action: "set_rating", rating: rating === n ? null : n })}
          >
            {rating !== undefined && n <= rating ? "★" : "☆"}
          </Button>
        ))}
        {rating !== undefined && rating > 0 && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void apply({ action: "set_rating", rating: null })}
          >
            {t("library.inspector.noRating")}
          </Button>
        )}
        {rating === undefined && <span className="muted">{t("common.mixed")}</span>}
      </div>
    </>
  );
}

/** Caratteristica di colore: bianco e nero, colore, colore virato, seppia, non indicato. */
export function ColorTraitRow({
  apply,
  colorTrait,
}: {
  apply: Apply;
  /** `undefined`: valori diversi tra le foto selezionate. */
  colorTrait: ColorTrait | null | undefined;
}) {
  const { t } = useTranslation();
  return (
    <>
      <label className="muted" htmlFor="inspector-color-trait">
        {t("library.inspector.colorTrait")}
      </label>
      <select
        id="inspector-color-trait"
        className="field"
        value={colorTrait === undefined ? "mixed" : (colorTrait ?? "")}
        onChange={(e) =>
          void apply({
            action: "set_color_trait",
            color_trait: (e.target.value || null) as ColorTrait | null,
          })
        }
      >
        {colorTrait === undefined && (
          <option value="mixed" disabled>
            {t("common.mixed")}
          </option>
        )}
        <option value="">{t("library.inspector.colorTraits.none")}</option>
        {COLOR_TRAITS.map((c) => (
          <option key={c} value={c}>
            {t(`library.inspector.colorTraits.${c}`)}
          </option>
        ))}
      </select>
    </>
  );
}

const SLICE: Record<DatePrecision, number> = { year: 4, month: 7, day: 10 };

/** Data impostata a mano come "AAAA", "AAAA-MM" o "AAAA-MM-GG". */
function manualPeriod(dates: MediaDates | null | undefined): string | null {
  if (!dates?.manualAt || !dates.manualPrecision) return null;
  return dates.manualAt.slice(0, SLICE[dates.manualPrecision]);
}

function monthName(m: number) {
  const loc = i18n.language === "en" ? "en-GB" : "it-IT";
  return new Intl.DateTimeFormat(loc, { month: "long", timeZone: "UTC" }).format(
    new Date(Date.UTC(2000, m - 1, 1)),
  );
}

const pad = (n: number) => String(n).padStart(2, "0");

function validDay(y: number, m: number, d: number) {
  return d >= 1 && d <= new Date(Date.UTC(y, m, 0)).getUTCDate();
}

/** Editor della data con precisione giorno/mese/anno. */
function DateEditor({
  initial,
  precision: initialPrecision,
  apply,
  close,
}: {
  initial: string | null;
  precision: DatePrecision | null;
  apply: Apply;
  close: () => void;
}) {
  const { t } = useTranslation();
  const [y0, m0, d0] = (initial ?? "").split("-");
  const [precision, setPrecision] = useState<DatePrecision>(initialPrecision ?? "day");
  const [year, setYear] = useState(y0 ?? "");
  const [month, setMonth] = useState(m0 ?? "01");
  const [day, setDay] = useState(d0 ?? "01");
  const valid =
    /^\d{4}$/.test(year) &&
    Number(year) >= 1 &&
    (precision !== "day" || validDay(+year, +month, +day));
  const value =
    precision === "year"
      ? year
      : precision === "month"
        ? `${year}-${month}`
        : `${year}-${month}-${pad(+day)}`;
  return (
    <>
      <SegmentedControl<DatePrecision>
        label={t("library.inspector.datePrecision")}
        value={precision}
        onChange={setPrecision}
        options={[
          { value: "day", label: t("library.inspector.precision.day") },
          { value: "month", label: t("library.inspector.precision.month") },
          { value: "year", label: t("library.inspector.precision.year") },
        ]}
      />
      <div className="inspector-date-fields">
        {precision === "day" && (
          <input
            className="field"
            type="number"
            min={1}
            max={31}
            aria-label={t("library.inspector.dateDay")}
            value={day}
            onChange={(e) => setDay(e.target.value)}
          />
        )}
        {precision !== "year" && (
          <select
            className="field"
            aria-label={t("library.inspector.dateMonth")}
            value={month}
            onChange={(e) => setMonth(e.target.value)}
          >
            {Array.from({ length: 12 }, (_, i) => (
              <option key={i} value={pad(i + 1)}>
                {monthName(i + 1)}
              </option>
            ))}
          </select>
        )}
        <input
          className="field"
          type="number"
          min={1}
          max={9999}
          aria-label={t("library.inspector.dateYear")}
          value={year}
          onChange={(e) => setYear(e.target.value)}
        />
      </div>
      <div className="inspector-actions">
        <Button size="sm" onClick={close}>
          {t("common.cancel")}
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={!valid}
          onClick={async () => {
            await apply({ action: "set_manual_date", value, precision });
            close();
          }}
        >
          {t("library.inspector.setDate")}
        </Button>
      </div>
    </>
  );
}

/** "Data di scatto" con la fonte e il ripristino della data del file. */
export function CapturedRow({
  apply,
  dates,
  captured,
}: {
  apply: Apply;
  dates: MediaDates | null;
  /** Data del file già formattata. */
  captured: string | null;
}) {
  const { t } = useTranslation();
  const manual = manualPeriod(dates);
  const label = t("library.inspector.captured");
  const shown = manual
    ? t("library.inspector.manualDate", { period: fmtPeriod(manual) })
    : captured;
  return (
    <>
      <span className="muted">{label}</span>
      <InlineEdit label={label} value={shown}>
        {(close) => (
          <>
            <DateEditor
              initial={manual}
              precision={dates?.manualPrecision ?? null}
              apply={apply}
              close={close}
            />
            {manual && (
              <Button
                size="sm"
                onClick={async () => {
                  await apply({ action: "clear_manual_date" });
                  close();
                }}
              >
                {t("library.inspector.restoreDate")}
              </Button>
            )}
          </>
        )}
      </InlineEdit>
    </>
  );
}
