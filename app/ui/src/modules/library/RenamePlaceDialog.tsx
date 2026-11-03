import { BusyButton } from "../../components/feedback/BusyButton";
// Rinomina di un luogo dal facet "Luogo": conferma con il numero di foto e
// avviso di unione se il nuovo nome esiste già.
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type FacetValue } from "../../lib/ipc";
import { notifyUndoable } from "../../store/undo";
import { useLibrary } from "../../store/library";

export function RenamePlaceDialog({
  from,
  count,
  places,
  onClose,
}: {
  from: string;
  count: number;
  places: FacetValue[];
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [to, setTo] = useState(from);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const target = to.trim();
  const merge = target !== from && places.some((p) => p.value === target);
  const confirm = async () => {
    setBusy(true);
    setError("");
    try {
      await commands.renamePlace(from, target);
      // Il filtro sul vecchio nome segue la rinomina.
      const { filter, setFilter } = useLibrary.getState();
      if ((filter.places ?? []).includes(from))
        setFilter({
          places: Array.from(new Set((filter.places ?? []).map((p) => (p === from ? target : p)))),
        });
      await qc.invalidateQueries();
      notifyUndoable({
        message: t("place.renamed", { from, to: target }),
        run: async () => {
          await commands.undoBulk();
          await qc.invalidateQueries();
        },
      });
      onClose();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("place.renameTitle")}
      onKeyDown={(e) => e.key === "Escape" && onClose()}
      className="rename-place"
    >
      <strong>{t("place.renameTitle")}</strong>
      <span className="mono rename-place-from">{from}</span>
      <input
        className="field"
        aria-label={t("place.renameTo")}
        value={to}
        autoFocus
        onChange={(e) => setTo(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && target && target !== from && void confirm()}
      />
      <span className="rename-place-note">{t("place.renameCount", { count })}</span>
      {merge && <span className="rename-place-warn">{t("place.renameMerge")}</span>}
      {error && <p role="alert">{error}</p>}
      <div className="feature-actions feature-actions-tight">
        <BusyButton
          className="btn"
          disabled={busy || !target || target === from}
          onClick={async () => await confirm()}
        >
          {t("place.renameConfirm")}
        </BusyButton>
        <BusyButton className="btn btn-ghost" onClick={onClose}>
          {t("place.cancel")}
        </BusyButton>
      </div>
    </div>
  );
}
