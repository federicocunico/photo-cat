// Gestione delle importazioni, condivisa tra Libreria e pagina Importa:
// rimozione con riepilogo, riepilogo dell'aggiornamento, ricollegamento e
// rimozione dei mancanti, aggiornamento di una cartella.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { Button, ConfirmDialog, Dialog, FolderPicker, InlineAlert } from "../../components/ui";
import {
  commands,
  type ChangeGroup,
  type RelinkSummary,
  type Source,
  type UpdateSummary,
} from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { notify } from "../../store/toasts";
import { pickFolder, TERMINAL, useShowInLibrary } from "./sourceActions";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

/** Rimozione di una cartella dalla libreria, con il riepilogo di cosa si perde. */
export function RemoveSourceDialog({
  source,
  onClose,
  onRemoved,
}: {
  source: Source;
  onClose: () => void;
  onRemoved?: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const summary = useQuery({
    queryKey: ["removal-summary", source.id],
    queryFn: () => commands.sourceRemovalSummary(source.id),
    gcTime: 0,
  });
  const s = summary.data;
  const remove = async () => {
    setBusy(true);
    setError("");
    try {
      await commands.removeSource(source.id);
      const { filter, setFilter } = useLibrary.getState();
      if ((filter.sourceIds ?? []).includes(source.id))
        setFilter({ sourceIds: (filter.sourceIds ?? []).filter((x) => x !== source.id) });
      await qc.invalidateQueries();
      onRemoved?.();
      onClose();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && !busy && onClose()}
      title={
        <span className="import-path">{t("sources.removal.title", { path: source.path })}</span>
      }
      width={560}
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="danger" busy={busy} disabled={!s} onClick={remove}>
            {busy ? t("sources.removal.busy") : t("sources.removal.confirm")}
          </Button>
        </>
      }
    >
      <QueryView query={summary} isEmpty={() => false}>
        {(s) => (
          <ul className="import-removal-list">
            <li>{t("sources.removal.items", { count: s.items, n: fmtInt(s.items) })}</li>
            {s.edited > 0 && <li>{t("sources.removal.edited", { count: s.edited })}</li>}
            {s.catalogs.length > 0 && (
              <li>
                {t("sources.removal.catalogs")}{" "}
                {s.catalogs
                  .map((c) =>
                    t("sources.removal.catalogItems", { name: c.name, n: fmtInt(c.items) }),
                  )
                  .join(", ")}
              </li>
            )}
            {s.orphanedPeople.length > 0 && (
              <li>{t("sources.removal.people", { names: s.orphanedPeople.join(", ") })}</li>
            )}
            {s.jobs > 0 && <li>{t("sources.removal.jobs", { count: s.jobs })}</li>}
          </ul>
        )}
      </QueryView>
      <p className="muted">{t("sources.removal.disk")}</p>
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
    </Dialog>
  );
}

const GROUPS = ["new", "modified", "moved", "missing", "restored"] as const;

/** Riepilogo dell'ultimo aggiornamento, con i gruppi che aprono la Libreria. */
export function UpdateSummaryView({ summary }: { summary: UpdateSummary }) {
  const { t } = useTranslation();
  const show = useShowInLibrary();
  const groups = GROUPS.map((g) => [g, summary[g] as ChangeGroup] as const).filter(
    ([, grp]) => grp.count > 0,
  );
  const none = groups.length === 0;
  return (
    <div
      data-testid={`update-summary-${summary.sourceId}`}
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: "4px 12px",
        alignItems: "center",
        fontSize: 13,
      }}
    >
      {summary.folder && (
        <span className="mono muted" style={{ wordBreak: "break-all" }}>
          {t("sources.summary.folder", { path: summary.folder })}
        </span>
      )}
      {none && <span>{t("sources.summary.none")}</span>}
      {groups.map(([g, grp]) => {
        const label = t(`sources.summary.${g}`, { n: fmtInt(grp.count) });
        const complete = grp.ids.length === grp.count;
        return complete ? (
          <BusyButton
            key={g}
            type="button"
            className="btn btn-ghost"
            style={{
              padding: "0 4px",
              color: g === "missing" ? "var(--pc-warn)" : undefined,
              textDecoration: "underline",
            }}
            aria-label={t("sources.summary.show", { label })}
            onClick={() => show(grp.ids)}
          >
            {label}
          </BusyButton>
        ) : (
          <span key={g}>{label}</span>
        );
      })}
      {summary.unchanged > 0 && !none && (
        <span className="muted">
          {t("sources.summary.unchanged", { n: fmtInt(summary.unchanged) })}
        </span>
      )}
      {summary.failed > 0 && (
        <span style={{ color: "var(--pc-danger)" }}>
          {t("sources.summary.failed", { n: fmtInt(summary.failed) })}
        </span>
      )}
    </div>
  );
}

/** Conferma della rimozione di tutti i mancanti di una cartella; l'esito è un avviso. */
export function RemoveMissingDialog({
  sourceId,
  missing,
  onClose,
}: {
  sourceId: number;
  missing: number;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  return (
    <ConfirmDialog
      open
      onOpenChange={(o) => !o && onClose()}
      title={t("sources.missingDialog.title")}
      message={t("sources.missingDialog.body", { count: missing })}
      confirmLabel={t("sources.missingDialog.confirm")}
      count={missing}
      files="untouched"
      onConfirm={async () => {
        // Un errore lascia il dialogo aperto e compare come avviso.
        const done = await commands.removeMissing(sourceId, true);
        notify({ tone: "ok", message: t("sources.missingDialog.done", { count: done }) });
        await qc.invalidateQueries();
      }}
    />
  );
}

/** Ricollegamento dei mancanti: cartella precedente proposta, nuova cartella e risultato. */
export function RelinkDialog({
  source,
  missing,
  onClose,
}: {
  source: Source;
  missing: number;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [oldFolder, setOldFolder] = useState<string | null>(null);
  const [newFolder, setNewFolder] = useState("");
  const [jobId, setJobId] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [note, setNote] = useState("");
  const proposed = useQuery({
    queryKey: ["missing-folder", source.id],
    queryFn: () => commands.missingFolder(source.id),
    gcTime: 0,
  });
  const job = useLive((s) => (jobId === null ? undefined : s.jobs[jobId]));
  const finished = job !== undefined && TERMINAL.includes(job.state);
  const result = useQuery({
    queryKey: ["relink", source.id, jobId],
    queryFn: () => commands.lastRelink(source.id),
    enabled: finished,
  });
  const r: RelinkSummary | null | undefined =
    result.data && result.data.jobId === jobId ? result.data : null;
  const old = oldFolder ?? proposed.data ?? source.path;
  const start = async () => {
    setError("");
    try {
      setJobId(await commands.relinkMissing(source.id, old, newFolder.trim()));
    } catch (e) {
      setError(errorText(e));
    }
  };
  const addSource = async () => {
    if (!r) return;
    try {
      await commands.addSource({
        path: r.newFolder,
        recursive: source.recursive,
        includeFormats: source.includeFormats,
        ignorePatterns: source.ignorePatterns,
        watch: source.watch,
      });
      setNote(t("sources.relinkDialog.sourceAdded"));
      await qc.invalidateQueries();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const moveSource = async () => {
    if (!r) return;
    try {
      await commands.moveSource(source.id, r.newFolder);
      setNote(t("sources.relinkDialog.sourceMoved"));
      await qc.invalidateQueries();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const sameRoot = (a: string, b: string) =>
    a.replace(/[\\/]+$/, "").toLowerCase() === b.replace(/[\\/]+$/, "").toLowerCase();
  return (
    <Dialog
      open
      onOpenChange={(o) => !o && onClose()}
      title={t("sources.relinkDialog.title")}
      description={t("sources.relinkDialog.intro", { count: missing })}
      width={560}
      footer={
        <>
          <Button onClick={onClose}>
            {jobId === null ? t("common.cancel") : t("common.close")}
          </Button>
          {jobId === null && (
            <Button variant="primary" disabled={!newFolder.trim() || !old.trim()} onClick={start}>
              {t("sources.relinkDialog.start")}
            </Button>
          )}
        </>
      }
    >
      <FolderPicker
        label={t("sources.relinkDialog.oldFolder")}
        value={old}
        disabled={jobId !== null}
        pick={() => pickFolder(old)}
        onChange={setOldFolder}
      />
      <FolderPicker
        label={t("sources.relinkDialog.newFolder")}
        value={newFolder}
        disabled={jobId !== null}
        pick={() => pickFolder()}
        onChange={setNewFolder}
      />
      {jobId !== null && !r && <p role="status">{t("sources.relinkDialog.running")}</p>}
      {r && (
        <div role="status" className="import-relink-result">
          <span>
            {t("sources.relinkDialog.result", {
              found: fmtInt(r.relinked + r.byHash),
              byPath: fmtInt(r.relinked),
              byHash: fmtInt(r.byHash),
            })}
          </span>
          {r.stillMissing > 0 && (
            <span className="import-warn">
              {t("sources.relinkDialog.still", { count: r.stillMissing })}
            </span>
          )}
          {r.ambiguous > 0 && (
            <span className="muted">
              {t("sources.relinkDialog.ambiguous", { count: r.ambiguous })}
            </span>
          )}
          {r.outsideSources && <span>{t("sources.relinkDialog.outside")}</span>}
          {r.outsideSources && !note && (
            <div>
              {r.sourceComplete && sameRoot(r.oldFolder, source.path) ? (
                <Button onClick={moveSource}>{t("sources.relinkDialog.moveSource")}</Button>
              ) : (
                <Button onClick={addSource}>{t("sources.relinkDialog.addSource")}</Button>
              )}
            </div>
          )}
          {note && <span>{note}</span>}
        </div>
      )}
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
    </Dialog>
  );
}
