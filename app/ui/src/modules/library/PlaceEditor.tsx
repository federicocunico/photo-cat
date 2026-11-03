import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
// Luogo modificabile: campo con suggerimenti (luoghi della libreria e località
// del dataset geografico) e ripristino del luogo automatico. Le modifiche
// passano dal job bulk "set_place", annullabile come le altre.
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type PlaceInput,
  type PlaceSuggestion,
  type Target,
} from "../../lib/ipc";
import { usePlaceAction } from "./usePlaceAction";

/** Campo del luogo con suggerimenti durante la digitazione. */
export function PlaceField({
  value,
  onChange,
  onPick,
  autoFocus,
}: {
  value: string;
  onChange: (v: string) => void;
  onPick: (s: PlaceSuggestion) => void;
  autoFocus?: boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const prefix = value.trim();
  const suggestions = useQuery({
    queryKey: ["place-suggest", prefix],
    queryFn: () => commands.suggestPlaces(prefix),
    enabled: isTauri() && open && prefix.length > 0,
    staleTime: 30_000,
  });
  const list = (suggestions.data ?? []).filter((s) => s.name !== value);
  return (
    <div className="place-field">
      <input
        className="field"
        aria-label={t("place.label")}
        placeholder={t("place.placeholder")}
        value={value}
        autoFocus={autoFocus}
        onChange={(e) => {
          onChange(e.target.value);
          setOpen(true);
        }}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && prefix.length > 0 && (
        <QueryView query={suggestions} isEmpty={() => false}>
          {() => null}
        </QueryView>
      )}
      {open && list.length > 0 && (
        <ul role="listbox" aria-label={t("place.suggestions")} className="place-suggest">
          {list.map((s) => (
            <li
              key={`${s.name}/${s.country ?? ""}`}
              role="option"
              aria-selected={false}
              tabIndex={-1}
              className="place-suggest-item"
              onMouseDown={(e) => {
                e.preventDefault();
                onPick(s);
                setOpen(false);
              }}
            >
              {s.name}{" "}
              <span className="muted">{s.inLibrary ? t("place.fromLibrary") : s.country}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * Luogo nell'ispettore: correzione manuale o ripristino del luogo automatico, su una
 * foto o su tutte quelle selezionate (`target`, `count`).
 */
export function PlaceEditor({
  target,
  count = 1,
  name,
  source,
  onDone,
}: {
  target: Target;
  count?: number;
  name: string | null;
  source: string | null;
  /** Chiamata dopo aver avviato la modifica (chiude la modifica sul campo). */
  onDone?: () => void;
}) {
  const { t } = useTranslation();
  const [value, setValue] = useState(name ?? "");
  const [picked, setPicked] = useState<PlaceSuggestion | null>(null);
  const { apply, error } = usePlaceAction();
  const run = async (place: PlaceInput | null) => {
    await apply(target, place, count);
    onDone?.();
  };
  useEffect(() => {
    setValue(name ?? "");
    setPicked(null);
  }, [name, source]);
  const trimmed = value.trim();
  const input = (): PlaceInput =>
    picked && picked.name === trimmed
      ? { name: picked.name, country: picked.country, lat: picked.lat, lon: picked.lon }
      : { name: trimmed };
  return (
    <div className="place-editor">
      <div className="feature-actions feature-actions-tight">
        <PlaceField
          autoFocus
          value={value}
          onChange={(v) => setValue(v)}
          onPick={(s) => {
            setValue(s.name);
            setPicked(s);
          }}
        />
        <BusyButton
          className="btn"
          disabled={!trimmed || (trimmed === (name ?? "") && source === "manual")}
          onClick={() => run(input())}
        >
          {t("place.set")}
        </BusyButton>
        {(source === "manual" || count > 1) && (
          <BusyButton className="btn" onClick={() => run(null)}>
            {t("place.reset")}
          </BusyButton>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
