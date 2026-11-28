// Valutazione a stelle, caratteristica di colore e data di scatto dell'Inspector
// (github-issues-39-47 2.1). Le modifiche passano dal bulk con "Annulla operazione".
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, InlineEdit, SegmentedControl } from "../../components/ui";
import type { BulkAction, ColorTrait, DatePrecision, MediaDates } from "../../lib/ipc";
import { fmtPeriod } from "../../lib/format";
import i18n from "../../i18n";
import { initialDateState, manualDateOf, type ManualDate } from "./manualDate";

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
  common,
  count,
  apply,
  close,
}: {
  /** Data manuale comune (o della sola foto). */
  common: ManualDate | null;
  /** Foto su cui agisce: con più foto e senza data comune i campi partono vuoti. */
  count: number;
  apply: Apply;
  close: () => void;
}) {
  const { t } = useTranslation();
  const [initial] = useState(() =>
    initialDateState(count, common, { precision: "day", year: "", month: "01", day: "01" }),
  );
  const [precision, setPrecision] = useState<DatePrecision>(initial.precision);
  const [year, setYear] = useState(initial.year);
  const [month, setMonth] = useState(initial.month);
  const [day, setDay] = useState(initial.day);
  const validMonth = /^\d{2}$/.test(month) && +month >= 1 && +month <= 12;
  const valid =
    /^\d{4}$/.test(year) &&
    Number(year) >= 1 &&
    (precision === "year" || validMonth) &&
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
            {month === "" && (
              <option value="" disabled>
                —
              </option>
            )}
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

/**
 * "Data di scatto" con il ripristino della data del file. Con più foto (`count > 1`)
 * mostra la data comune o "Valori diversi" e agisce su tutta la selezione.
 */
export function CapturedRow({
  apply,
  dates,
  captured,
  count = 1,
  restorable,
}: {
  apply: Apply;
  /** `undefined`: date diverse tra le foto selezionate (o non note). */
  dates: MediaDates | null | undefined;
  /** Data del file già formattata. */
  captured: string | null;
  /** Foto su cui agisce la riga. */
  count?: number;
  /** "Ripristina data del file" disponibile; di norma se c'è una data manuale. */
  restorable?: boolean;
}) {
  const { t } = useTranslation();
  const manual = manualDateOf(dates);
  const label = t("library.inspector.captured");
  const shown =
    dates === undefined
      ? t("common.mixed")
      : manual
        ? t("library.inspector.manualDate", { period: fmtPeriod(manual.value) })
        : captured;
  return (
    <>
      <span className="muted">{label}</span>
      <InlineEdit label={label} value={shown}>
        {(close) => (
          <>
            <DateEditor common={manual} count={count} apply={apply} close={close} />
            {(restorable ?? manual != null) && (
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
