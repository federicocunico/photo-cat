// Selettore del valore di un gruppo di catalogazione con "Nuova voce…": il
// campo per il nome compare sul posto (Invio conferma, Esc annulla).
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Group } from "../../lib/ipc";
import { useLabelName } from "./labelName";

const NEW = "__new__";

export function CategoryPicker({
  group,
  value,
  onChange,
  onCreate,
  ariaLabel,
}: {
  group: Group | undefined;
  value: string;
  onChange: (value: string) => void;
  /** Crea (o riusa) la voce con questo nome e la assegna. */
  onCreate: (label: string) => Promise<void>;
  ariaLabel: string;
}) {
  const { t } = useTranslation();
  const names = useLabelName();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const cancel = () => {
    setEditing(false);
    setName("");
    setError("");
  };
  const confirm = async () => {
    if (!name.trim()) {
      setError(t("entries.required"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      await onCreate(name.trim());
      cancel();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const options = (group?.categories ?? []).filter((c) => c.enabled || c.id === value);
  if (editing)
    return (
      <span style={{ display: "inline-flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
        <input
          className="field"
          autoFocus
          aria-label={t("entries.newEntryName")}
          aria-invalid={error ? true : undefined}
          value={name}
          disabled={busy}
          onChange={(e) => {
            setName(e.target.value);
            setError("");
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void confirm();
            } else if (e.key === "Escape") {
              e.preventDefault();
              cancel();
            }
          }}
        />
        <button type="button" className="btn" disabled={busy} onClick={() => void confirm()}>
          {t("entries.confirm")}
        </button>
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={cancel}>
          {t("entries.cancel")}
        </button>
        {error && (
          <span role="alert" style={{ color: "var(--pc-danger)", fontSize: 13 }}>
            {error}
          </span>
        )}
      </span>
    );
  return (
    <select
      className="field"
      aria-label={ariaLabel}
      value={value}
      onChange={(e) => (e.target.value === NEW ? setEditing(true) : onChange(e.target.value))}
    >
      <option value="">—</option>
      {options.map((c) => (
        <option key={c.id} value={c.id}>
          {names.label(group?.id ?? "", c.id)}
        </option>
      ))}
      {group && <option value={NEW}>{t("entries.newEntry")}</option>}
    </select>
  );
}
