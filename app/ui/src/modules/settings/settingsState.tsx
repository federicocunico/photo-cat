// Stato condiviso delle Impostazioni (design D9):
// - le preferenze si salvano appena modificate (debounce 400 ms) con "Salvato" accanto
//   al campo; la cache ["config"] è aggiornata subito e torna all'ultimo valore salvato
//   se il motore rifiuta la modifica;
// - soglie, tassonomia e percorso dei dati restano in una bozza "da applicare", con la
//   barra "Ripristina / Applica" e l'anteprima dell'effetto.
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { commands, type Settings, type Taxonomy } from "../../lib/ipc";
import { cleanSettings } from "./cleanSettings";
import {
  SAVE_DELAY,
  SettingsCtx,
  useSettings,
  type Ctx,
  type FieldStatus,
} from "./settingsContext";

export function SettingsProvider({
  config,
  taxonomy,
  children,
}: {
  config: Settings;
  taxonomy: Taxonomy | undefined;
  children: ReactNode;
}) {
  const qc = useQueryClient();
  const [draft, setDraft] = useState<Settings>(config);
  const [tax, setTax] = useState<Taxonomy | null>(taxonomy ?? null);
  const [status, setStatus] = useState<Record<string, FieldStatus>>({});
  const [highlight, setHighlight] = useState<string | null>(null);
  // Ultima configurazione accettata dal motore: si torna qui se un salvataggio fallisce.
  const confirmed = useRef<Settings>(config);
  const queue = useRef<{ field: string; after?: (n: Settings, p: Settings) => void }[]>([]);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clearTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  useEffect(() => {
    if (taxonomy && tax == null) setTax(taxonomy);
  }, [taxonomy, tax]);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
      Object.values(clearTimers.current).forEach(clearTimeout);
    },
    [],
  );

  const mark = useCallback((fields: string[], s: FieldStatus) => {
    setStatus((prev) => {
      const next = { ...prev };
      for (const f of fields) next[f] = s;
      return next;
    });
    if (s.state !== "saved") return;
    for (const f of fields) {
      clearTimeout(clearTimers.current[f]);
      clearTimers.current[f] = setTimeout(
        () =>
          setStatus((prev) => {
            if (prev[f]?.state !== "saved") return prev;
            const next = { ...prev };
            delete next[f];
            return next;
          }),
        2500,
      );
    }
  }, []);

  const flush = useCallback(async () => {
    timer.current = null;
    const items = queue.current;
    queue.current = [];
    if (!items.length) return;
    const fields = [...new Set(items.map((i) => i.field))];
    const prev = confirmed.current;
    const next = qc.getQueryData<Settings>(["config"]) ?? prev;
    try {
      await commands.saveSettings(cleanSettings(next));
      confirmed.current = next;
      mark(fields, { state: "saved" });
      for (const i of items) i.after?.(next, prev);
    } catch (e) {
      qc.setQueryData(["config"], confirmed.current);
      mark(fields, {
        state: "error",
        message: (e as { detail?: string } | null)?.detail ?? String(e),
      });
    }
  }, [qc, mark]);

  const autosave = useCallback<Ctx["autosave"]>(
    (field, patch, after) => {
      const current = qc.getQueryData<Settings>(["config"]) ?? confirmed.current;
      qc.setQueryData(["config"], patch(current));
      queue.current.push({ field, after });
      mark([field], { state: "saving" });
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => void flush(), SAVE_DELAY);
    },
    [qc, mark, flush],
  );

  // Dopo "Applica" la configurazione salvata è anche quella confermata.
  useEffect(() => {
    if (!queue.current.length) confirmed.current = config;
  }, [config]);

  return (
    <SettingsCtx.Provider
      value={{
        saved: config,
        draft,
        setDraft,
        savedTax: taxonomy,
        tax,
        setTax,
        autosave,
        status,
        highlight,
        setHighlight,
      }}
    >
      {children}
    </SettingsCtx.Provider>
  );
}

/** "Salvato" (o l'errore) accanto al campo. */
export function SaveState({ field }: { field: string }) {
  const { t } = useTranslation();
  const s = useSettings().status[field];
  return (
    <span className="settings-save-state" aria-live="polite">
      {s?.state === "saved" && (
        <span className="settings-saved">
          <Icon.Check size={14} aria-hidden="true" />
          {t("settings.savedField")}
        </span>
      )}
      {s?.state === "error" && (
        <span role="alert" className="settings-save-error">
          {t("settings.saveError", { error: s.message })}
        </span>
      )}
    </span>
  );
}

/**
 * Un'impostazione: etichetta, controllo, suggerimento e stato del salvataggio. `id`
 * è anche la destinazione della ricerca, che evidenzia il campo.
 */
export function Setting({
  id,
  label,
  hint,
  children,
  inline,
  labelFor,
}: {
  id: string;
  label: ReactNode;
  /** Id del controllo, se l'etichetta lo nomina (altrimenti il controllo ha il suo nome). */
  labelFor?: string;
  hint?: ReactNode;
  children: ReactNode;
  /** Controllo accanto all'etichetta (interruttori). */
  inline?: boolean;
}) {
  const { highlight } = useSettings();
  return (
    <div
      className="settings-field"
      data-setting={id}
      data-highlight={highlight === id || undefined}
      data-inline={inline || undefined}
    >
      <div className="settings-field-head">
        {labelFor ? (
          <label className="settings-field-label" htmlFor={labelFor}>
            {label}
          </label>
        ) : (
          <span className="settings-field-label">{label}</span>
        )}
        <SaveState field={id} />
      </div>
      <div className="settings-field-control">{children}</div>
      {hint && <p className="settings-field-hint muted">{hint}</p>}
    </div>
  );
}
