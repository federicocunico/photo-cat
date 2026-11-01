// Luogo modificabile: campo con suggerimenti (luoghi della libreria e località
// del dataset geografico) e ripristino del luogo automatico. Le modifiche
// passano dal job bulk "set_place", annullabile come le altre.
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type PlaceInput, type PlaceSuggestion } from "../../lib/ipc";
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
    <div style={{ position: "relative", flex: 1, minWidth: 0 }}>
      <input
        className="field"
        style={{ width: "100%" }}
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
      {open && list.length > 0 && (
        <ul
          role="listbox"
          aria-label={t("place.suggestions")}
          style={{
            position: "absolute",
            zIndex: 5,
            left: 0,
            right: 0,
            margin: 0,
            padding: 4,
            listStyle: "none",
            background: "var(--pc-raised)",
            border: "1px solid var(--pc-border)",
            borderRadius: 4,
          }}
        >
          {list.map((s) => (
            <li
              key={`${s.name}/${s.country ?? ""}`}
              role="option"
              aria-selected={false}
              tabIndex={-1}
              onMouseDown={(e) => {
                e.preventDefault();
                onPick(s);
                setOpen(false);
              }}
              style={{ padding: "3px 6px", cursor: "pointer", fontSize: 13 }}
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

/** Luogo nell'ispettore: modifica come correzione manuale o ripristino automatico. */
export function PlaceEditor({
  id,
  name,
  source,
}: {
  id: number;
  name: string | null;
  source: string | null;
}) {
  const { t } = useTranslation();
  const [value, setValue] = useState(name ?? "");
  const [picked, setPicked] = useState<PlaceSuggestion | null>(null);
  const { apply, error } = usePlaceAction();
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
    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
      <div className="feature-actions" style={{ margin: 0 }}>
        <PlaceField
          value={value}
          onChange={(v) => setValue(v)}
          onPick={(s) => {
            setValue(s.name);
            setPicked(s);
          }}
        />
        <button
          className="btn"
          disabled={!trimmed || (trimmed === (name ?? "") && source === "manual")}
          onClick={() => void apply([id], input())}
        >
          {t("place.set")}
        </button>
        {source === "manual" && (
          <button className="btn" onClick={() => void apply([id], null)}>
            {t("place.reset")}
          </button>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
