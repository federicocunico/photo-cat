import { useId, useMemo, useState, type KeyboardEvent } from "react";

export type PickerOption = {
  value: string;
  label: string;
  /** Testo secondario a destra (es. numero di foto). */
  hint?: string;
};

/**
 * Selettore con ricerca da tastiera (desktop-shell, "Selettori delle azioni sulla
 * selezione"): si scrive per filtrare, frecce per scegliere, Invio per confermare.
 * Con `createLabel` il testo scritto si può usare come valore nuovo.
 */
export function SearchPicker({
  label,
  placeholder,
  options,
  onPick,
  createLabel,
  emptyLabel,
  limit = 50,
}: {
  /** Nome del campo per il lettore di schermo. */
  label: string;
  placeholder?: string;
  options: PickerOption[];
  /** `created` è vero quando il valore è il testo scritto e non un'opzione esistente. */
  onPick: (value: string, created: boolean) => unknown;
  createLabel?: (text: string) => string;
  emptyLabel?: string;
  limit?: number;
}) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const text = query.trim();
  const items = useMemo(() => {
    const q = text.toLocaleLowerCase();
    const found = options.filter((o) => o.label.toLocaleLowerCase().includes(q)).slice(0, limit);
    const exact = found.some((o) => o.label.toLocaleLowerCase() === q);
    const create =
      createLabel && text && !exact
        ? [{ value: text, label: createLabel(text), created: true as const }]
        : [];
    return [...found.map((o) => ({ ...o, created: false as const })), ...create];
  }, [options, text, createLabel, limit]);
  const current = Math.min(active, Math.max(0, items.length - 1));

  const pick = (i: number) => {
    const item = items[i];
    if (item) void onPick(item.value, item.created);
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!items.length) return;
      const n = items.length;
      setActive((current + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    } else if (e.key === "Enter") {
      e.preventDefault();
      pick(current);
    } else if (e.key === "Home" || e.key === "End") {
      if (!items.length) return;
      e.preventDefault();
      setActive(e.key === "Home" ? 0 : items.length - 1);
    }
  };

  return (
    <div className="ui-picker">
      <input
        className="field ui-picker-input"
        role="combobox"
        aria-label={label}
        aria-expanded={items.length > 0}
        aria-controls={`${id}-list`}
        aria-autocomplete="list"
        aria-activedescendant={items.length ? `${id}-${current}` : undefined}
        placeholder={placeholder}
        value={query}
        autoFocus
        onChange={(e) => {
          setQuery(e.target.value);
          setActive(0);
        }}
        onKeyDown={onKeyDown}
      />
      <ul id={`${id}-list`} role="listbox" aria-label={label} className="ui-picker-list">
        {items.map((item, i) => (
          <li
            key={`${item.created}-${item.value}`}
            id={`${id}-${i}`}
            role="option"
            aria-selected={i === current}
            className="ui-picker-option"
            data-created={item.created || undefined}
            onMouseDown={(e) => e.preventDefault()}
            onMouseEnter={() => setActive(i)}
            onClick={() => pick(i)}
          >
            <span>{item.label}</span>
            {"hint" in item && item.hint && <span className="muted">{item.hint}</span>}
          </li>
        ))}
      </ul>
      {items.length === 0 && emptyLabel && <p className="muted ui-picker-empty">{emptyLabel}</p>}
    </div>
  );
}
