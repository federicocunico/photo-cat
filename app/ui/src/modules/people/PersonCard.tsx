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
import { Button, InlineAlert } from "../../components/ui";
import { notifyPeopleUndo, refreshPeople } from "./peopleUndo";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import { IdentityEditor } from "./IdentityEditor";
import { MergeDialog } from "./MergeDialog";
import { Transfer } from "./Transfer";
import { WhoIs } from "./WhoIs";
import type { GroupListTarget } from "./GroupList";
import "./people.css";

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
    <div className="people-stack people-stack--gap-xs">
      <span className="label people-label">{t("people.cover")}</span>
      <div className="people-inline">
        <span
          aria-hidden
          className="people-cover people-cover--md"
          style={{
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
        <span className="muted people-muted-sm">{t("people.coverHint")}</span>
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
    (group?.identityUid ? (everyone.find((r) => r.uid === group.identityUid) ?? null) : null);

  const refresh = () => refreshPeople(qc);

  /** Correzione del gruppo; `done` è l'avviso con "Annulla operazione". */
  const correct = async (c: Correction, done: string) => {
    setBusy(true);
    setError("");
    try {
      await commands.correctPerson(c);
      await refresh();
      notifyPeopleUndo(qc, done);
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
      void correct({ action: "ignore", group: group.id }, t("people.ignoredDone"));
      return true;
    },
  }));

  // Sezione Persone (e creazione): editor identità + Transfer.
  if (section === "people" || creating) {
    return (
      <div className="people-stack people-stack--gap-xl people-stack--scroll">
        <QueryView
          query={identities}
          isEmpty={() => !creating && !row}
          error={t("people.failed")}
          empty={<p className="muted people-muted-reset">{t("people.pickGroup")}</p>}
        >
          {() => null}
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
        <div className="people-stack people-stack--gap-xs">
          <span className="label people-label">{t("people.transfer")}</span>
          <Transfer onDone={() => void refresh()} />
        </div>
        {identities.data?.storePath && (
          <p role="note" className="muted people-muted-reset people-muted-sm">
            {t("identities.appNote")} {t("identities.storeAt")} {identities.data.storePath}
          </p>
        )}
      </div>
    );
  }

  if (target?.kind === "singles") {
    return <p className="muted people-muted-reset">{t("people.singlesCard")}</p>;
  }

  if (!group || target?.kind !== "group") {
    return <p className="muted people-muted-reset">{t("people.pickGroup")}</p>;
  }

  return (
    <div className="people-stack people-stack--gap-xl people-stack--scroll">
      {section === "review" && (
        <div className="people-stack people-stack--gap-xs">
          <span className="label people-label">{t("identities.whoIs")}</span>
          <WhoIs
            person={group.id}
            linked={group.identityUid ? (group.name ?? null) : null}
            onLinked={onLinked}
            inputRef={whoIsRef}
          />
          <span className="muted people-muted-sm">{t("people.whoIsHint")}</span>
        </div>
      )}

      <CoverPreview group={group} />
      {selectedFaceIds.length === 1 && (
        <Button
          disabled={busy}
          onClick={() =>
            correct(
              { action: "set_cover", group: group.id, face: selectedFaceIds[0]! },
              t("people.coverDone"),
            )
          }
        >
          {t("people.useAsCover")}
        </Button>
      )}

      <div className="people-stack people-stack--gap">
        <span className="label people-label">{t("people.groupActions")}</span>
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
        <Button disabled={busy || !mergeTo} onClick={() => setMergeOpen(true)}>
          {t("people.merge")}
        </Button>
        {section !== "ignored" && (
          <>
            <Button
              disabled={busy}
              onClick={() =>
                correct({ action: "ignore", group: group.id }, t("people.ignoredDone"))
              }
            >
              {t("people.ignore")}
            </Button>
            <span className="muted people-muted-sm">{t("people.ignoreHint")}</span>
          </>
        )}
        {section === "ignored" && (
          <Button
            disabled={busy}
            onClick={() =>
              correct({ action: "restore", group: group.id }, t("people.restoredDone"))
            }
          >
            {t("people.restore")}
          </Button>
        )}
      </div>
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
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
