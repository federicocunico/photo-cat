import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, InlineAlert } from "../../components/ui";
import { commands, type Correction, type FaceView, type GroupRow } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import { notifyPeopleUndo } from "./peopleUndo";
import "./people.css";

function Avatar({ cover, size = 64 }: { cover: FaceView | null; size?: number }) {
  const [failed, setFailed] = useState(false);
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  return (
    <span
      aria-hidden
      className="people-cover"
      style={{
        width: size,
        height: size,
        backgroundImage: placeholder && !src ? `url("${placeholder}")` : undefined,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          onError={() => setFailed(true)}
          className="people-cover-img"
        />
      )}
    </span>
  );
}

function labelOf(g: GroupRow, t: (k: string, o?: Record<string, unknown>) => string) {
  return g.name ?? t("people.groupOfFaces", { count: g.faces });
}

/**
 * Dialogo di unione: anteprima, scelta del nome se più di uno ne ha uno. L'unione compare
 * poi come avviso "Persone unite · Annulla operazione" (anche Ctrl+Z).
 */
export function MergeDialog({
  sources,
  target,
  onClose,
  onMerged,
}: {
  sources: GroupRow[];
  target: GroupRow;
  onClose: () => void;
  onMerged: (to: number) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const parties = [target, ...sources.filter((s) => s.id !== target.id)];
  const named = parties.filter((g) => g.name);
  const needNameChoice = named.length >= 2;
  const [keepId, setKeepId] = useState(target.id);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const survivor = parties.find((g) => g.id === keepId) ?? target;
  const absorbed = parties.filter((g) => g.id !== survivor.id);
  const absorbedFaces = absorbed.reduce((n, g) => n + g.faces, 0);
  const primarySource = absorbed[0] ?? sources[0]!;

  const confirm = async () => {
    if (!absorbed.length) {
      onClose();
      return;
    }
    setBusy(true);
    setError("");
    try {
      const correction: Correction =
        absorbed.length === 1
          ? { action: "merge", from: absorbed[0]!.id, to: survivor.id }
          : { action: "merge_many", from: absorbed.map((g) => g.id), to: survivor.id };
      await commands.correctPerson(correction);
      notifyPeopleUndo(qc, t("people.mergedDone"));
      onMerged(survivor.id);
      onClose();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && !busy && onClose()}
      title={t("people.mergeDialogTitle")}
      footer={
        <>
          <Button variant="ghost" disabled={busy} onClick={onClose}>
            {t("people.mergeCancel")}
          </Button>
          <Button
            variant="primary"
            disabled={busy || absorbed.length === 0}
            onClick={() => confirm()}
          >
            {t("people.mergeInto", { name: labelOf(survivor, t) })}
          </Button>
        </>
      }
    >
      <div className="people-merge">
        <div className="people-merge-preview">
          <div className="people-merge-party">
            <Avatar cover={primarySource.cover} />
            <b>{labelOf(primarySource, t)}</b>
            <span className="muted people-muted-sm">
              {t("people.photos", { count: primarySource.photos })}
            </span>
          </div>
          <span aria-hidden className="people-merge-arrow">
            →
          </span>
          <div className="people-merge-party">
            <Avatar cover={survivor.cover} />
            <b>{labelOf(survivor, t)}</b>
            <span className="muted people-muted-sm">
              {t("people.photos", { count: survivor.photos })}
            </span>
          </div>
        </div>

        <p className="people-merge-body">
          {t("people.mergeDialogBody", {
            count: absorbedFaces,
            name: labelOf(survivor, t),
          })}
        </p>

        {needNameChoice && (
          <fieldset className="people-merge-fieldset">
            <legend className="people-merge-legend">{t("people.mergeKeepName")}</legend>
            <div className="people-merge-options">
              {named.map((g) => (
                <label key={g.id} className="people-check people-check--gap">
                  <input
                    type="radio"
                    name="merge-keep-name"
                    checked={keepId === g.id}
                    onChange={() => setKeepId(g.id)}
                  />
                  {g.name}
                </label>
              ))}
            </div>
          </fieldset>
        )}

        {error && (
          <InlineAlert tone="danger" title={t("people.failed")}>
            {error}
          </InlineAlert>
        )}
      </div>
    </Dialog>
  );
}
