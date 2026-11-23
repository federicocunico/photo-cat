import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type GroupRow, type IdentityInput, type IdentityRow } from "../../lib/ipc";
import { Icon } from "../../components/icons";
import { Button, ConfirmDialog, InlineAlert, Menu } from "../../components/ui";
import { notify, notifyError } from "../../store/toasts";
import { Cover } from "./Cover";
import { PersonPicker, type PickerChoice } from "./PersonPicker";
import { openPersonPhotos } from "./openPersonPhotos";
import "./people.css";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const KEY = ["identities"] as const;

type Draft = { name: string; shortName: string; group: string; aliases: string; threshold: number };

const toDraft = (r: IdentityRow | null, def: number): Draft => ({
  name: r?.name ?? "",
  shortName: r?.shortName ?? "",
  group: r?.group ?? "",
  aliases: r?.aliases.join(", ") ?? "",
  threshold: r?.threshold ?? def,
});

const toInput = (d: Draft, def: number): IdentityInput => ({
  name: d.name.trim(),
  shortName: d.shortName.trim() || null,
  group: d.group.trim() || null,
  threshold: Math.abs(d.threshold - def) < 1e-6 ? null : d.threshold,
  aliases: d.aliases
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean),
});

/** "26 foto in questa libreria · Famiglia · conosciuta anche come «Gio»". */
function summary(row: IdentityRow, t: (k: string, o?: Record<string, unknown>) => string) {
  return [
    row.person != null && row.photos > 0
      ? t("people.photosInLibrary", { count: row.photos })
      : t("people.noPhotosInLibrary"),
    row.group,
    row.aliases.length
      ? t("identities.alsoKnownAs", { names: row.aliases.map((a) => `«${a}»`).join(", ") })
      : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

/**
 * Scheda di una persona (identity-registry, "Gestione delle identità"): intestazione con la
 * copertina, "Mostra in Libreria" e il menu ⋯ (Unisci a…, Esporta, Elimina persona…); dettagli
 * salvati appena modificati, con "Salvato" accanto. Per una persona nuova: il modulo con "Crea".
 */
export function IdentityEditor({
  row,
  creating,
  groups,
  defaultThreshold,
  cover = null,
  onSaved,
  onDeleted,
}: {
  row: IdentityRow | null;
  creating: boolean;
  /** Tutte le persone (non più usato: l'unione passa dal selettore). */
  all?: IdentityRow[];
  groups: string[];
  defaultThreshold: number;
  /** Copertina del gruppo della persona nella libreria aperta. */
  cover?: GroupRow["cover"];
  onSaved: (uid: string) => void;
  onDeleted: () => void;
  /** Mantenuto per compatibilità: la scheda ha sempre il layout della colonna. */
  compact?: boolean;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [draft, setDraft] = useState<Draft>(toDraft(row, defaultThreshold));
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [mergeOpen, setMergeOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Ultimi dettagli salvati: uscire da un campo non modificato non salva di nuovo.
  const lastSaved = useRef("");
  useEffect(() => {
    setDraft(toDraft(row, defaultThreshold));
    lastSaved.current = JSON.stringify(toInput(toDraft(row, defaultThreshold), defaultThreshold));
    setError("");
    setSaved(false);
    setMergeOpen(false);
    setConfirmDelete(false);
  }, [row?.uid, creating, defaultThreshold]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(
    () => () => {
      if (savedTimer.current) clearTimeout(savedTimer.current);
    },
    [],
  );
  const refresh = () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: KEY }),
      qc.invalidateQueries({ queryKey: ["identity-faces"] }),
      qc.invalidateQueries({ queryKey: ["people"] }),
      qc.invalidateQueries({ queryKey: ["people-groups"] }),
      qc.invalidateQueries({ queryKey: ["people-sections"] }),
    ]);
  // Blocco sincrono: un doppio clic o un doppio invio non manda due volte la stessa azione.
  const locked = useRef(false);
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<unknown>) => {
    if (locked.current) return;
    locked.current = true;
    setBusy(true);
    setError("");
    try {
      await fn();
      await refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };

  /** Salvataggio immediato dei dettagli (persona esistente): "Salvato" per un momento. */
  const autosave = (next: Draft) => {
    if (!row || creating) return;
    const input = toInput(next, defaultThreshold);
    const key = JSON.stringify(input);
    if (!input.name || key === lastSaved.current) return;
    void run(async () => {
      await commands.saveIdentity(row.uid, input);
      lastSaved.current = key;
      setSaved(true);
      if (savedTimer.current) clearTimeout(savedTimer.current);
      savedTimer.current = setTimeout(() => setSaved(false), 2500);
    });
  };

  const create = () =>
    run(async () => {
      const uid = await commands.saveIdentity(null, toInput(draft, defaultThreshold));
      onSaved(uid);
    });

  const exportPerson = async () => {
    if (!row) return;
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({
      defaultPath: `${row.shortName || row.name}.photocat-people.json`,
      filters: [{ name: "photo-cat", extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    try {
      await commands.exportIdentities(path, true, [row.uid]);
      notify({ tone: "ok", message: t("identities.exportedOne", { name: row.name }) });
    } catch (e) {
      notifyError(e);
    }
  };

  const field = (
    key: Exclude<keyof Draft, "threshold">,
    label: string,
    extra: Record<string, unknown> = {},
    hint?: string,
  ) => (
    <label className="people-field-col">
      <span className="people-field-label">{label}</span>
      <input
        className="field people-field-input"
        value={draft[key]}
        onChange={(e) => setDraft({ ...draft, [key]: e.target.value })}
        onBlur={() => autosave(draft)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !creating) autosave(draft);
        }}
        {...extra}
      />
      {hint && <span className="muted people-muted-sm">{hint}</span>}
    </label>
  );

  if (!row && !creating) return <p className="muted">{t("identities.none")}</p>;

  const details = (
    <>
      {field("name", t("identities.name"), { required: true })}
      {field("shortName", t("identities.shortName"), {
        placeholder: draft.name.trim().split(/\s+/)[0] ?? "",
      })}
      {field(
        "group",
        t("identities.group"),
        { list: "pc-identity-groups" },
        t("identities.groupHint"),
      )}
      <datalist id="pc-identity-groups">
        {groups.map((g) => (
          <option key={g} value={g} />
        ))}
      </datalist>
      {field("aliases", t("identities.aliases"), { placeholder: t("identities.aliasesHint") })}
      {/* Riconoscimento come sensibilità, senza valori numerici (ui-foundations). */}
      <label className="people-field-col">
        <span className="people-field-label">{t("identities.threshold")}</span>
        <input
          type="range"
          min={0}
          max={100}
          value={Math.round(draft.threshold * 100)}
          aria-label={t("identities.threshold")}
          aria-valuetext={t(
            draft.threshold < defaultThreshold - 0.02
              ? "identities.sensitivity.more"
              : draft.threshold > defaultThreshold + 0.02
                ? "identities.sensitivity.precise"
                : "identities.sensitivity.default",
          )}
          onChange={(e) => setDraft({ ...draft, threshold: Number(e.target.value) / 100 })}
          onPointerUp={() => autosave(draft)}
          onKeyUp={() => autosave(draft)}
          onBlur={() => autosave(draft)}
        />
        <span className="people-scale muted people-muted-sm" aria-hidden="true">
          <span>{t("identities.sensitivity.more")}</span>
          <span>{t("identities.sensitivity.precise")}</span>
        </span>
      </label>
    </>
  );

  if (creating || !row) {
    return (
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void create();
        }}
        className="people-stack people-stack--gap-md"
      >
        <h2 className="people-card-title">{t("identities.new")}</h2>
        {details}
        <button
          className="btn btn-primary"
          type="submit"
          disabled={!draft.name.trim()}
          aria-busy={busy || undefined}
        >
          {t("identities.create")}
        </button>
        {error && (
          <InlineAlert tone="danger" title={t("people.failed")}>
            {error}
          </InlineAlert>
        )}
      </form>
    );
  }

  const mergeName = (c: PickerChoice) => c.name;
  return (
    <div className="people-stack people-stack--gap-lg">
      <div className="people-person-head">
        <Cover cover={cover} size="md" />
        <div className="people-person-title">
          <h2 className="people-card-title">{row.name}</h2>
          <span className="muted people-muted-sm">{summary(row, t)}</span>
        </div>
        <Menu
          align="end"
          trigger={
            <button
              type="button"
              className="btn btn-ghost icon-btn"
              aria-label={t("identities.moreActions", { name: row.name })}
            >
              <Icon.More size={16} aria-hidden="true" />
            </button>
          }
          items={[
            { label: t("people.mergeTo"), onSelect: () => setMergeOpen(true) },
            { label: t("identities.export"), onSelect: () => void exportPerson() },
            { kind: "separator" },
            {
              label: t("identities.deletePerson"),
              danger: true,
              onSelect: () => setConfirmDelete(true),
            },
          ]}
        />
      </div>
      {row.person != null && row.photos > 0 && (
        <Button
          icon={<Icon.Library size={14} />}
          onClick={() => openPersonPhotos(row.person!).catch((e) => notifyError(e))}
        >
          {t("identities.showInLibrary")}
        </Button>
      )}
      <section className="people-stack people-stack--gap-md" aria-labelledby={`details-${row.uid}`}>
        <div className="people-inline">
          <h3 id={`details-${row.uid}`} className="section-title">
            {t("identities.details")}
          </h3>
          <span role="status" className="muted people-muted-sm people-saved">
            {saved ? t("settings.savedField") : null}
          </span>
        </div>
        {details}
      </section>
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
      {mergeOpen && (
        <PersonPicker
          mode="identities"
          title={t("people.picker.mergeTitle")}
          intro={t("identities.mergeIntro", { name: row.name })}
          danger
          describe={(c: PickerChoice) => {
            if (c.kind !== "identity" || c.uid === row.uid)
              throw new Error(t("identities.mergeSame"));
            return {
              title: t("identities.mergeTitle", { from: row.name, into: c.name }),
              body: t("identities.mergeConfirm", { from: row.name, into: c.name }),
            };
          }}
          confirmLabel={(c: PickerChoice) => t("identities.mergeButton", { into: mergeName(c) })}
          run={async (c: PickerChoice) => {
            if (c.kind !== "identity") return;
            await commands.mergeIdentities(row.uid, c.uid);
            await refresh();
            onSaved(c.uid);
          }}
          onClose={() => setMergeOpen(false)}
        />
      )}
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t("identities.deleteTitle", { name: row.name })}
        message={t("identities.deleteConfirm", { name: row.name })}
        confirmLabel={t("identities.deleteButton")}
        onConfirm={() =>
          run(async () => {
            await commands.deleteIdentity(row.uid);
            onDeleted();
          })
        }
      />
    </div>
  );
}
