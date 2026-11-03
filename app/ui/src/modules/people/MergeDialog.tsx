import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, InlineAlert } from "../../components/ui";
import { commands, type Correction, type FaceView, type GroupRow } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import { notifyPeopleUndo } from "./peopleUndo";

function Avatar({ cover, size = 64 }: { cover: FaceView | null; size?: number }) {
  const [failed, setFailed] = useState(false);
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  return (
    <span
      aria-hidden
      style={{
        display: "block",
        width: size,
        height: size,
        borderRadius: "50%",
        overflow: "hidden",
        backgroundColor: "var(--pc-thumb-bg)",
        backgroundImage: placeholder && !src ? `url("${placeholder}")` : undefined,
        backgroundSize: "cover",
        backgroundPosition: "center",
        flexShrink: 0,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          onError={() => setFailed(true)}
          style={{ width: "100%", height: "100%", objectFit: "cover" }}
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
      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr auto 1fr",
            gap: 8,
            alignItems: "center",
          }}
        >
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 6,
              alignItems: "center",
              textAlign: "center",
            }}
          >
            <Avatar cover={primarySource.cover} />
            <b>{labelOf(primarySource, t)}</b>
            <span className="muted" style={{ fontSize: 12 }}>
              {t("people.photos", { count: primarySource.photos })}
            </span>
          </div>
          <span aria-hidden style={{ fontSize: 20, color: "var(--pc-text-2)" }}>
            →
          </span>
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 6,
              alignItems: "center",
              textAlign: "center",
            }}
          >
            <Avatar cover={survivor.cover} />
            <b>{labelOf(survivor, t)}</b>
            <span className="muted" style={{ fontSize: 12 }}>
              {t("people.photos", { count: survivor.photos })}
            </span>
          </div>
        </div>

        <p style={{ margin: 0 }}>
          {t("people.mergeDialogBody", {
            count: absorbedFaces,
            name: labelOf(survivor, t),
          })}
        </p>

        {needNameChoice && (
          <fieldset
            style={{
              border: "1px solid var(--pc-border)",
              borderRadius: 6,
              padding: 10,
              margin: 0,
            }}
          >
            <legend style={{ padding: "0 6px" }}>{t("people.mergeKeepName")}</legend>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {named.map((g) => (
                <label key={g.id} style={{ display: "flex", gap: 8, alignItems: "center" }}>
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
