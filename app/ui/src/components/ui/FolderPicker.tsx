import { useId, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { Button } from "./Button";

/** Dialogo di sistema per scegliere una cartella; sostituibile nei test. */
export async function chooseFolder(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const p = await open({ directory: true, multiple: false });
  return typeof p === "string" ? p : null;
}

/** Percorso di un elemento trascinato (WebView di Tauri: `File.path`; altrimenti testo). */
function droppedPath(e: DragEvent): string | null {
  const file = e.dataTransfer.files?.[0] as (File & { path?: string }) | undefined;
  if (file?.path) return file.path;
  const text = e.dataTransfer.getData("text/plain").trim();
  return text || null;
}

/**
 * Scelta di una cartella: percorso leggibile (modificabile a mano), pulsante "Scegli…"
 * e trascinamento di una cartella sul campo.
 */
export function FolderPicker({
  label,
  value,
  onChange,
  hint,
  pick = chooseFolder,
  disabled,
}: {
  label: string;
  value: string;
  onChange: (path: string) => void;
  hint?: string;
  /** Apre il selettore di sistema; restituisce `null` se l'utente annulla. */
  pick?: () => Promise<string | null>;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const id = useId();
  const [over, setOver] = useState(false);
  return (
    <div
      className={["ui-folder", over ? "ui-folder-over" : ""].filter(Boolean).join(" ")}
      onDragOver={(e) => {
        if (disabled) return;
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        if (disabled) return;
        e.preventDefault();
        setOver(false);
        const p = droppedPath(e);
        if (p) onChange(p);
      }}
    >
      <label htmlFor={id} className="ui-field-label">
        {label}
      </label>
      <div className="ui-folder-row">
        <Icon.Import size={16} />
        <input
          id={id}
          className="field ui-folder-input"
          value={value}
          placeholder={t("common.noFolder")}
          aria-describedby={hint ? `${id}-hint` : undefined}
          disabled={disabled}
          spellCheck={false}
          onChange={(e) => onChange(e.target.value)}
        />
        <Button
          disabled={disabled}
          onClick={async () => {
            const p = await pick();
            if (p) onChange(p);
          }}
        >
          {t("common.browse")}
        </Button>
      </div>
      {hint && (
        <p id={`${id}-hint`} className="ui-field-hint muted">
          {hint}
        </p>
      )}
    </div>
  );
}
