import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type MergeEntry } from "../../lib/ipc";
import { fmtDate, fmtDateTime } from "../../lib/format";
import { Button, Dialog, InlineAlert } from "../../components/ui";
import { notify } from "../../store/toasts";
import { Cover } from "./Cover";
import { refreshPeopleFast } from "./peopleUndo";
import { QUEUE_KEY } from "./ReviewQueue";
import "./people.css";

export const MERGES_KEY = ["merges"] as const;
const LIMIT = 500;

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const cover = (id: number | null) => (id != null ? { id } : null);

/** "Gruppo di 3 volti → Gioia", "2 gruppi → Persona senza nome". */
function useEntryText() {
  const { t } = useTranslation();
  return (e: MergeEntry) => {
    const from =
      e.sources.length === 1
        ? (e.sources[0]!.name ?? t("people.merges.groupOf", { count: e.sources[0]!.faces }))
        : t("people.merges.groups", { count: e.sources.length });
    return `${from} → ${e.targetName ?? t("people.unnamedPlain")}`;
  };
}

/**
 * Persone › "Unioni recenti" (face-recognition, "Storico delle unioni"): le unioni degli ultimi
 * 90 giorni, manuali, automatiche o dalla coda, con le copertine dei gruppi e "Annulla". Un'unione
 * automatica annullata non si ripete alle analisi successive.
 */
export function RecentMerges({
  open,
  person = null,
  onClose,
}: {
  open: boolean;
  /** Filtro iniziale: le unioni che riguardano questa persona. */
  person?: number | null;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const text = useEntryText();
  const [filter, setFilter] = useState<number | null>(person);
  const [busy, setBusy] = useState<number | null>(null);
  const [error, setError] = useState("");
  const merges = useQuery({
    queryKey: MERGES_KEY,
    queryFn: () => commands.listMerges(null, LIMIT),
    enabled: isTauri() && open,
  });
  const all = useMemo(() => merges.data ?? [], [merges.data]);
  // Persone con nome che compaiono nello storico, per il filtro.
  const people = useMemo(() => {
    const seen = new Map<number, string>();
    for (const e of all) {
      if (e.targetName) seen.set(e.target, e.targetName);
      for (const s of e.sources) if (s.name) seen.set(s.person, s.name);
    }
    return [...seen].sort((a, b) => a[1].localeCompare(b[1]));
  }, [all]);
  const shown =
    filter == null
      ? all
      : all.filter((e) => e.target === filter || e.sources.some((s) => s.person === filter));

  const undo = async (e: MergeEntry) => {
    setBusy(e.id);
    setError("");
    try {
      const faces = await commands.undoMerge(e.id);
      notify({ tone: "ok", message: t("people.merges.undone", { count: faces, text: text(e) }) });
      await Promise.all([
        qc.invalidateQueries({ queryKey: MERGES_KEY }),
        qc.invalidateQueries({ queryKey: QUEUE_KEY }),
        refreshPeopleFast(qc),
      ]);
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => !o && onClose()}
      title={t("people.merges.title")}
      description={t("people.merges.intro")}
      width={640}
      footer={<Button onClick={onClose}>{t("common.close")}</Button>}
    >
      <div className="people-stack people-stack--gap-md">
        <label className="people-field-col">
          <span className="people-field-label">{t("people.merges.filter")}</span>
          <select
            className="field"
            value={filter ?? ""}
            onChange={(e) => setFilter(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">{t("people.merges.everyone")}</option>
            {people.map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
        </label>
        {error && (
          <InlineAlert tone="danger" title={t("people.failed")} onDismiss={() => setError("")}>
            {error}
          </InlineAlert>
        )}
        {merges.data && shown.length === 0 && (
          <p className="muted people-muted-reset">{t("people.merges.empty")}</p>
        )}
        <ul className="people-merges" aria-label={t("people.merges.title")}>
          {shown.map((e) => (
            <li
              key={e.id}
              className="people-merge-row"
              data-undone={e.undoneAt != null || undefined}
            >
              <span className="people-merge-covers" aria-hidden>
                {e.sources.slice(0, 3).map((s) => (
                  <Cover key={s.person} cover={cover(s.cover)} />
                ))}
                <span className="muted">→</span>
                <Cover cover={cover(e.targetCover)} />
              </span>
              <span className="people-merge-text">
                <span className="people-merge-title">{text(e)}</span>
                <span className="muted people-muted-sm">
                  {[
                    fmtDateTime(e.at),
                    t(`people.merges.origin.${e.origin}`),
                    t("people.facesCount", { count: e.faces }),
                  ].join(" · ")}
                </span>
              </span>
              {e.undoneAt != null ? (
                <span className="muted people-muted-sm">
                  {t("people.merges.undoneOn", { date: fmtDate(e.undoneAt) })}
                </span>
              ) : (
                <Button
                  size="sm"
                  disabled={busy === e.id}
                  disabledReason={e.undoable ? undefined : t("people.merges.notUndoable")}
                  onClick={() => void undo(e)}
                  aria-label={t("people.merges.undoLabel", { text: text(e) })}
                >
                  {t("people.merges.undo")}
                </Button>
              )}
            </li>
          ))}
        </ul>
      </div>
    </Dialog>
  );
}
