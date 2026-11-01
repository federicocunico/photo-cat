// Gestione delle importazioni, condivisa tra Libreria e pagina Import:
// rimozione con riepilogo, riepilogo dell'aggiornamento, ricollegamento e
// rimozione dei mancanti, aggiornamento di una cartella.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type React from "react";
import { useTranslation } from "react-i18next";
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
import { pickFolder, TERMINAL, useShowInLibrary } from "./sourceActions";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

export function Modal({
  label,
  onClose,
  children,
}: {
  label: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 60,
        background: "rgba(0,0,0,0.35)",
        display: "grid",
        placeItems: "center",
      }}
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={label}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
        style={{
          width: "min(560px, 92vw)",
          maxHeight: "86vh",
          overflow: "auto",
          border: "1px solid var(--pc-border)",
          borderRadius: 8,
          padding: 20,
          background: "var(--pc-raised)",
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        {children}
      </div>
    </div>
  );
}

/** Conferma della rimozione di una sorgente con il riepilogo di cosa si perde. */
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
  const title = t("sources.removal.title", { path: source.path });
  return (
    <Modal label={title} onClose={busy ? () => {} : onClose}>
      <strong style={{ wordBreak: "break-all" }}>{title}</strong>
      {summary.error && <p role="alert">{errorText(summary.error)}</p>}
      {s && (
        <ul
          style={{ margin: 0, paddingLeft: 18, display: "flex", flexDirection: "column", gap: 4 }}
        >
          <li>{t("sources.removal.items", { count: s.items, n: fmtInt(s.items) })}</li>
          {s.edited > 0 && <li>{t("sources.removal.edited", { count: s.edited })}</li>}
          {s.catalogs.length > 0 && (
            <li>
              {t("sources.removal.catalogs")}{" "}
              {s.catalogs
                .map((c) => t("sources.removal.catalogItems", { name: c.name, n: fmtInt(c.items) }))
                .join(", ")}
            </li>
          )}
          {s.orphanedPeople.length > 0 && (
            <li>{t("sources.removal.people", { names: s.orphanedPeople.join(", ") })}</li>
          )}
          {s.jobs > 0 && <li>{t("sources.removal.jobs", { count: s.jobs })}</li>}
        </ul>
      )}
      <span className="muted" style={{ fontSize: 13 }}>
        {t("sources.removal.disk")}
      </span>
      {error && <p role="alert">{error}</p>}
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={onClose}>
          {t("sources.cancel")}
        </button>
        <button
          type="button"
          className="btn btn-danger"
          disabled={busy || !s}
          onClick={() => void remove()}
        >
          {busy ? t("sources.removal.busy") : t("sources.removal.confirm")}
        </button>
      </div>
    </Modal>
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
          <button
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
          </button>
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

/** Conferma della rimozione di tutti i mancanti di una sorgente. */
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [done, setDone] = useState<number | null>(null);
  const title = t("sources.missingDialog.title");
  const remove = async () => {
    setBusy(true);
    setError("");
    try {
      setDone(await commands.removeMissing(sourceId, true));
      await qc.invalidateQueries();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal label={title} onClose={onClose}>
      <strong>{title}</strong>
      {done === null ? (
        <p style={{ margin: 0 }}>{t("sources.missingDialog.body", { count: missing })}</p>
      ) : (
        <p role="status" style={{ margin: 0 }}>
          {t("sources.missingDialog.done", { count: done })}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button type="button" className="btn btn-ghost" onClick={onClose}>
          {done === null ? t("sources.cancel") : t("sources.close")}
        </button>
        {done === null && (
          <button
            type="button"
            className="btn btn-danger"
            disabled={busy}
            onClick={() => void remove()}
          >
            {t("sources.missingDialog.confirm")}
          </button>
        )}
      </div>
    </Modal>
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
  const title = t("sources.relinkDialog.title");
  const sameRoot = (a: string, b: string) =>
    a.replace(/[\\/]+$/, "").toLowerCase() === b.replace(/[\\/]+$/, "").toLowerCase();
  return (
    <Modal label={title} onClose={onClose}>
      <strong>{title}</strong>
      <span>{t("sources.relinkDialog.intro", { count: missing })}</span>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {t("sources.relinkDialog.oldFolder")}
        <input
          className="field mono"
          value={old}
          disabled={jobId !== null}
          onChange={(e) => setOldFolder(e.target.value)}
        />
      </label>
      <label htmlFor="relink-new" style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {t("sources.relinkDialog.newFolder")}
      </label>
      <div style={{ display: "flex", gap: 8 }}>
        <input
          id="relink-new"
          className="field mono"
          style={{ flex: 1 }}
          value={newFolder}
          disabled={jobId !== null}
          onChange={(e) => setNewFolder(e.target.value)}
        />
        <button
          type="button"
          className="btn"
          disabled={jobId !== null}
          onClick={async () => {
            const p = await pickFolder();
            if (p) setNewFolder(p);
          }}
        >
          {t("sources.relinkDialog.browse")}
        </button>
      </div>
      {jobId !== null && !r && <span role="status">{t("sources.relinkDialog.running")}</span>}
      {r && (
        <div role="status" style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <span>
            {t("sources.relinkDialog.result", {
              found: fmtInt(r.relinked + r.byHash),
              byPath: fmtInt(r.relinked),
              byHash: fmtInt(r.byHash),
            })}
          </span>
          {r.stillMissing > 0 && (
            <span style={{ color: "var(--pc-warn)" }}>
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
            <div style={{ display: "flex", gap: 8 }}>
              {r.sourceComplete && sameRoot(r.oldFolder, source.path) ? (
                <button type="button" className="btn" onClick={() => void moveSource()}>
                  {t("sources.relinkDialog.moveSource")}
                </button>
              ) : (
                <button type="button" className="btn" onClick={() => void addSource()}>
                  {t("sources.relinkDialog.addSource")}
                </button>
              )}
            </div>
          )}
          {note && <span>{note}</span>}
        </div>
      )}
      {error && <p role="alert">{error}</p>}
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button type="button" className="btn btn-ghost" onClick={onClose}>
          {jobId === null ? t("sources.cancel") : t("sources.close")}
        </button>
        {jobId === null && (
          <button
            type="button"
            className="btn btn-primary"
            disabled={!newFolder.trim() || !old.trim()}
            onClick={() => void start()}
          >
            {t("sources.relinkDialog.start")}
          </button>
        )}
      </div>
    </Modal>
  );
}
