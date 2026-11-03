import { useQuery, useQueryClient } from "@tanstack/react-query";
import { forwardRef, useImperativeHandle, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Correction,
  type GroupRow,
  type IdentityRow,
  type PeopleSection,
} from "../../lib/ipc";
import { QueryView } from "../../components/feedback/QueryView";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import { IdentityEditor } from "./IdentityEditor";
import { MergeDialog } from "./MergeDialog";
import { Transfer } from "./Transfer";
import { WhoIs } from "./WhoIs";
import type { GroupListTarget } from "./GroupList";

export type PersonCardHandle = {
  focusWhoIs: () => void;
  focusMergeSelect: () => void;
  openMergeIfReady: () => boolean;
  ignoreGroup: () => boolean;
};

function CoverPreview({ group }: { group: GroupRow }) {
  const { t } = useTranslation();
  const cover = group.cover;
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  const [failed, setFailed] = useState(false);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span
        className="label"
        style={{ fontSize: 12, letterSpacing: "0.06em", color: "var(--pc-text-2)", fontWeight: 600 }}
      >
        {t("people.cover")}
      </span>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <span
          aria-hidden
          style={{
            width: 56,
            height: 56,
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
        <span className="muted" style={{ fontSize: 12 }}>
          {t("people.coverHint")}
        </span>
      </div>
    </div>
  );
}

/** Scheda destra: Chi è? / copertina / unione / ignora, oppure IdentityEditor per le persone. */
export const PersonCard = forwardRef<
  PersonCardHandle,
  {
    section: PeopleSection;
    target: GroupListTarget;
    group: GroupRow | null;
    identity: IdentityRow | null;
    creating: boolean;
    selectedFaceIds: number[];
    mergeCandidates: GroupRow[];
    onLinked: (person: number) => void;
    onIdentitySaved: (uid: string) => void;
    onIdentityDeleted: () => void;
    onCreatingChange: (v: boolean) => void;
  }
>(function PersonCard(
  {
    section,
    target,
    group,
    identity,
    creating,
    selectedFaceIds,
    mergeCandidates,
    onLinked,
    onIdentitySaved,
    onIdentityDeleted,
    onCreatingChange,
  },
  ref,
) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [mergeTo, setMergeTo] = useState("");
  const [mergeOpen, setMergeOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const whoIsRef = useRef<HTMLInputElement>(null);
  const mergeSelectRef = useRef<HTMLSelectElement>(null);

  const identities = useQuery({
    queryKey: ["identities", ""],
    queryFn: () => commands.listIdentities(null),
    enabled: isTauri() && (section === "people" || creating),
  });

  const everyone = identities.data?.identities ?? [];
  const idGroups = identities.data?.groups ?? [];
  const def = identities.data?.defaultThreshold ?? 0.363;
  const row =
    identity ??
    (group?.identityUid ? everyone.find((r) => r.uid === group.identityUid) ?? null : null);

  const refresh = () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: ["people"] }),
      qc.invalidateQueries({ queryKey: ["people-groups"] }),
      qc.invalidateQueries({ queryKey: ["people-sections"] }),
      qc.invalidateQueries({ queryKey: ["group-faces"] }),
      qc.invalidateQueries({ queryKey: ["faces"] }),
      qc.invalidateQueries({ queryKey: ["identities"] }),
    ]);

  const correct = async (c: Correction) => {
    setBusy(true);
    setError("");
    try {
      await commands.correctPerson(c);
      await refresh();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  useImperativeHandle(ref, () => ({
    focusWhoIs() {
      whoIsRef.current?.focus();
    },
    focusMergeSelect() {
      mergeSelectRef.current?.focus();
    },
    openMergeIfReady() {
      if (!mergeTo || !group) {
        mergeSelectRef.current?.focus();
        return false;
      }
      setMergeOpen(true);
      return true;
    },
    ignoreGroup() {
      if (!group || section === "ignored" || busy) return false;
      void correct({ action: "ignore", group: group.id });
      return true;
    },
  }));

  // Sezione Persone (e creazione): editor identità + Transfer.
  if (section === "people" || creating) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: 16, overflow: "auto" }}>
        <QueryView
          isLoading={identities.isLoading && !creating && !row}
          isError={identities.isError}
          isEmpty={!creating && !row}
          loading={<p className="muted">{t("people.loading")}</p>}
          error={<p role="alert">{t("people.failed")}</p>}
          empty={
            <p className="muted" style={{ margin: 0 }}>
              {t("people.pickGroup")}
            </p>
          }
        >
          {null}
        </QueryView>
        {(creating || row) && (
          <IdentityEditor
            row={creating ? null : row}
            creating={creating}
            all={everyone}
            groups={idGroups}
            defaultThreshold={def}
            compact
            onSaved={(uid) => {
              onCreatingChange(false);
              onIdentitySaved(uid);
            }}
            onDeleted={() => {
              onCreatingChange(false);
              onIdentityDeleted();
            }}
          />
        )}
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <span
            className="label"
            style={{
              fontSize: 12,
              letterSpacing: "0.06em",
              color: "var(--pc-text-2)",
              fontWeight: 600,
            }}
          >
            {t("people.transfer")}
          </span>
          <Transfer onDone={() => void refresh()} />
        </div>
        {identities.data?.storePath && (
          <p role="note" className="muted" style={{ margin: 0, fontSize: 12 }}>
            {t("identities.appNote")} {t("identities.storeAt")}{" "}
            <code className="mono">{identities.data.storePath}</code>
          </p>
        )}
      </div>
    );
  }

  if (target?.kind === "singles") {
    return (
      <p className="muted" style={{ margin: 0 }}>
        {t("people.singlesCard")}
      </p>
    );
  }

  if (!group || target?.kind !== "group") {
    return (
      <p className="muted" style={{ margin: 0 }}>
        {t("people.pickGroup")}
      </p>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16, overflow: "auto" }}>
      {section === "review" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <span
            className="label"
            style={{
              fontSize: 12,
              letterSpacing: "0.06em",
              color: "var(--pc-text-2)",
              fontWeight: 600,
            }}
          >
            {t("identities.whoIs")}
          </span>
          <WhoIs
            person={group.id}
            linked={group.identityUid ? (group.name ?? null) : null}
            onLinked={onLinked}
            inputRef={whoIsRef}
          />
          <span className="muted" style={{ fontSize: 12 }}>
            {t("people.whoIsHint")}
          </span>
        </div>
      )}

      <CoverPreview group={group} />
      {selectedFaceIds.length === 1 && (
        <button
          className="btn"
          disabled={busy}
          onClick={() =>
            void correct({ action: "set_cover", group: group.id, face: selectedFaceIds[0]! })
          }
        >
          {t("people.useAsCover")}
        </button>
      )}

      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <span
          className="label"
          style={{
            fontSize: 12,
            letterSpacing: "0.06em",
            color: "var(--pc-text-2)",
            fontWeight: 600,
          }}
        >
          {t("people.groupActions")}
        </span>
        <select
          ref={mergeSelectRef}
          className="field"
          aria-label={t("people.mergeTarget")}
          value={mergeTo}
          onChange={(e) => setMergeTo(e.target.value)}
        >
          <option value="">{t("people.mergeWith")}</option>
          {mergeCandidates
            .filter((g) => g.id !== group.id)
            .map((g) => (
              <option key={g.id} value={g.id}>
                {g.name ?? t("people.groupOfFaces", { count: g.faces })}
              </option>
            ))}
        </select>
        <button
          className="btn"
          disabled={busy || !mergeTo}
          onClick={() => setMergeOpen(true)}
        >
          {t("people.merge")}
        </button>
        {section !== "ignored" && (
          <>
            <button
              className="btn"
              disabled={busy}
              onClick={() => void correct({ action: "ignore", group: group.id })}
            >
              {t("people.ignore")}
            </button>
            <span className="muted" style={{ fontSize: 12 }}>
              {t("people.ignoreHint")}
            </span>
          </>
        )}
        {section === "ignored" && (
          <button
            className="btn"
            disabled={busy}
            onClick={() => void correct({ action: "restore", group: group.id })}
          >
            {t("people.restore")}
          </button>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
      {mergeOpen && mergeTo && (
        <MergeDialog
          sources={[group]}
          target={
            mergeCandidates.find((g) => g.id === Number(mergeTo)) ?? {
              ...group,
              id: Number(mergeTo),
            }
          }
          onClose={() => setMergeOpen(false)}
          onMerged={(to) => {
            setMergeTo("");
            void refresh().then(() => onLinked(to));
          }}
        />
      )}
    </div>
  );
});
