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
import { notifyPeopleUndo, refreshPeople, refreshPeopleFast } from "./peopleUndo";
import { IdentityEditor } from "./IdentityEditor";
import { MergeDialog } from "./MergeDialog";
import { PersonPicker, type PickerChoice } from "./PersonPicker";
import { Transfer } from "./Transfer";
import { WhoIs } from "./WhoIs";
import type { GroupListTarget } from "./GroupList";
import { openPersonPhotos } from "./openPersonPhotos";
import { notifyError } from "../../store/toasts";
import { Icon } from "../../components/icons";
import { Cover } from "./Cover";
import { similarityLevel } from "./similarity";
import { withNewIdentity } from "./newIdentity";
import "./people.css";

export type PersonCardHandle = {
  focusWhoIs: () => void;
  /** "Unisci a…" (tasto U). */
  openMerge: () => boolean;
  ignoreGroup: () => boolean;
};

/**
 * Domanda in cima alla scheda quando il gruppo somiglia a una persona o a un altro gruppo
 * (face-recognition, "Suggerimento di identità nella scheda"): le due copertine, la somiglianza
 * in parole e "Sì" / "No".
 */
function MergeQuestion({
  group,
  busy,
  onYes,
  onNo,
}: {
  group: GroupRow;
  busy: boolean;
  onYes: () => void;
  onNo: () => void;
}) {
  const { t } = useTranslation();
  const hint = group.mergeHint!;
  const name = hint.otherName;
  return (
    <section className="people-question" aria-labelledby={`question-${group.id}`}>
      <div className="people-question-covers" aria-hidden>
        <Cover cover={group.cover} size="md" />
        <Cover cover={hint.otherCover} size="md" />
      </div>
      <h3 id={`question-${group.id}`} className="people-question-title">
        {name ? t("people.question.isName", { name }) : t("people.question.same")}
      </h3>
      <p className="muted people-muted-sm">
        {hint.score != null
          ? t(`people.question.similarity.${similarityLevel(hint.score)}`, {
              count: hint.otherFaces,
              name: name ?? t("people.unnamedPlain"),
            })
          : t("people.question.noScore")}
      </p>
      <div className="people-question-actions">
        <Button variant="primary" disabled={busy} onClick={onYes}>
          {name ? t("people.question.yesName", { name }) : t("people.question.yes")}
        </Button>
        <Button disabled={busy} onClick={onNo}>
          {t("people.question.no")}
        </Button>
      </div>
    </section>
  );
}

/** Scheda destra: domanda, "Chi è?" e azioni del gruppo, oppure l'editor della persona. */
export const PersonCard = forwardRef<
  PersonCardHandle,
  {
    section: PeopleSection;
    target: GroupListTarget;
    group: GroupRow | null;
    identity: IdentityRow | null;
    creating: boolean;
    onLinked: (person: number) => void;
    onIdentitySaved: (uid: string) => void;
    onIdentityDeleted: () => void;
    onCreatingChange: (v: boolean) => void;
    /** "Unioni recenti" filtrate per questa persona. */
    onShowMerges?: (person: number) => void;
  }
>(function PersonCard(
  {
    section,
    target,
    group,
    identity,
    creating,
    onLinked,
    onIdentitySaved,
    onIdentityDeleted,
    onCreatingChange,
    onShowMerges,
  },
  ref,
) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [mergeOpen, setMergeOpen] = useState(false);
  const [namedMerge, setNamedMerge] = useState<GroupRow | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const whoIsRef = useRef<HTMLInputElement>(null);

  const identities = useQuery({
    queryKey: ["identities", ""],
    queryFn: () => commands.listIdentities(null, null),
    enabled: isTauri() && (section === "people" || creating),
  });

  const everyone = identities.data?.identities ?? [];
  const idGroups = identities.data?.groups ?? [];
  const def = identities.data?.defaultThreshold ?? 0.363;
  const row =
    identity ??
    (group?.identityUid ? (everyone.find((r) => r.uid === group.identityUid) ?? null) : null);
  // Copertina della persona dall'elenco aggiornato (dopo "Usa come copertina" si ricarica);
  // per i gruppi senza nome quella del gruppo.
  const cover =
    (row && everyone.find((r) => r.uid === row.uid)?.cover) ?? row?.cover ?? group?.cover ?? null;

  const refresh = () => refreshPeople(qc);

  /** Correzione del gruppo; `done` è l'avviso con "Annulla operazione". */
  const correct = async (c: Correction, done: string, after?: () => void) => {
    setBusy(true);
    setError("");
    try {
      await commands.correctPerson(c);
      await refreshPeopleFast(qc);
      notifyPeopleUndo(qc, done);
      after?.();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  /** Unione del gruppo aperto in `to`: annullabile, quindi senza conferma (ui-foundations). */
  const mergeInto = (to: number, name: string | null) => {
    if (!group) return;
    // Due nomi diversi: si sceglie quale tenere nel dialogo dell'unione.
    if (group.name && name && group.name !== name) {
      setNamedMerge({ ...group, id: to, name });
      return;
    }
    void correct(
      { action: "merge", from: group.id, to },
      t("people.mergedDone", { count: 1, name: name ?? t("people.unnamedPlain") }),
      () => onLinked(to),
    );
  };

  useImperativeHandle(ref, () => ({
    focusWhoIs() {
      whoIsRef.current?.focus();
    },
    openMerge() {
      if (!group || section === "people") return false;
      setMergeOpen(true);
      return true;
    },
    ignoreGroup() {
      if (!group || section === "ignored" || busy) return false;
      void correct({ action: "ignore", group: group.id }, t("people.ignoredDone"));
      return true;
    },
  }));

  // Sezione Persone (e creazione): editor della persona + Transfer.
  if (section === "people" || creating) {
    return (
      <div className="people-stack people-stack--gap-xl people-stack--scroll">
        {/* "Scegli un gruppo o una persona" è già nella colonna dei volti: non si ripete qui. */}
        <QueryView
          query={identities}
          isEmpty={() => !creating && !row}
          error={t("people.failed")}
          empty={<></>}
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
            cover={creating ? null : cover}
            onSaved={(uid) => {
              onCreatingChange(false);
              onIdentitySaved(uid);
            }}
            onDeleted={() => {
              onCreatingChange(false);
              onIdentityDeleted();
            }}
            onShowMerges={onShowMerges}
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

  // Volti singoli e scartati: la scheda dice che cosa sono e come si sistemano.
  if (target?.kind === "singles")
    return <p className="muted people-muted-reset">{t("people.singlesCard")}</p>;
  if (target?.kind === "queue")
    return <p className="muted people-muted-reset">{t("people.queue.cardHint")}</p>;
  if (target?.kind === "discarded")
    return <p className="muted people-muted-reset">{t("people.discardedCard")}</p>;

  // Nessun gruppo aperto: l'invito a scegliere è nella colonna dei volti, non si ripete qui.
  if (!group || target?.kind !== "group") return null;

  const hint = section !== "ignored" ? group.mergeHint : null;

  return (
    <div className="people-stack people-stack--gap-xl people-stack--scroll">
      <div className="people-stack people-stack--gap-xs">
        <h2 className="people-card-title">{group.name ?? t("people.unnamedPlain")}</h2>
        <span className="muted people-muted-sm">
          {t("people.photosInLibrary", { count: group.photos })}
        </span>
      </div>

      {hint && (
        <MergeQuestion
          group={group}
          busy={busy}
          onYes={() => mergeInto(hint.otherId, hint.otherName)}
          onNo={() =>
            void correct(
              { action: "not_same", a: group.id, b: hint.otherId },
              t("people.notSameDone"),
            )
          }
        />
      )}

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

      <div className="people-stack people-stack--gap">
        <Button
          icon={<Icon.Library size={14} />}
          onClick={() => openPersonPhotos(group.id).catch((e) => notifyError(e))}
        >
          {t("people.showPhotos", { count: group.photos })}
        </Button>
        {section !== "ignored" && (
          <Button disabled={busy} onClick={() => setMergeOpen(true)}>
            {t("people.mergeGroup")}
          </Button>
        )}
        {section !== "ignored" && (
          <>
            <Button
              disabled={busy}
              onClick={() =>
                correct({ action: "ignore", group: group.id }, t("people.ignoredDone"))
              }
            >
              {t("people.ignoreGroup")}
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
      {mergeOpen && (
        <PersonPicker
          mode="destination"
          title={t("people.picker.mergeTitle")}
          intro={t("people.picker.mergeIntro", { count: 1 })}
          allowCreate
          exclude={[group.id]}
          run={async (c: PickerChoice) => {
            if (c.kind === "person") {
              mergeInto(c.person, c.name === t("people.unnamedPlain") ? null : c.name);
              return;
            }
            // Una persona senza volti in questa libreria (o nuova): il gruppo prende il suo nome.
            const link = (uid: string) => commands.linkPersonIdentity(group.id, uid);
            const person =
              c.kind === "identity" ? await link(c.uid) : await withNewIdentity(c.name, link);
            notifyPeopleUndo(qc, t("people.namedDone", { name: c.name }));
            await refreshPeopleFast(qc);
            onLinked(person);
          }}
          onClose={() => setMergeOpen(false)}
        />
      )}
      {namedMerge && (
        <MergeDialog
          sources={[group]}
          target={namedMerge}
          onClose={() => setNamedMerge(null)}
          onMerged={(to) => {
            setNamedMerge(null);
            void refresh().then(() => onLinked(to));
          }}
        />
      )}
    </div>
  );
});
