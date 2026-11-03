import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  isTauri,
  type GroupRow,
  type IdentityRow,
  type LocateFaceRequest,
  type PeopleSection,
} from "../../lib/ipc";
import { listAllGroups } from "./allGroups";
import { useLayout, type PeopleTarget } from "../../store/layout";
import { useLibrary } from "../../store/library";
import { FaceGrid, type FaceGridHandle, type FaceHighlight } from "./FaceGrid";
import { GroupList, type GroupListHandle, type GroupListTarget } from "./GroupList";
import { MergeDialog } from "./MergeDialog";
import { PersonCard, type PersonCardHandle } from "./PersonCard";
import { usePeopleHotkeys } from "./usePeopleHotkeys";

type Arrival = {
  highlight: FaceHighlight;
  mediaId: number;
  mediaName: string;
};

function stubGroup(id: number, section: PeopleSection, name: string | null = null): GroupRow {
  return {
    id,
    name,
    identityUid: null,
    section,
    faces: 0,
    goodFaces: 0,
    discardedFaces: 0,
    photos: 0,
    cover: null,
    ignoredAt: null,
    newFaces: 0,
    nameHint: null,
    mergeHint: null,
  };
}

async function loadGroup(section: PeopleSection, id: number): Promise<GroupRow> {
  return (await commands.getGroup(id)) ?? stubGroup(id, section);
}

function toLocateRequest(target: PeopleTarget): LocateFaceRequest | null {
  if ("face" in target) return { kind: "face", face_id: target.face };
  if ("media" in target)
    return { kind: "media", media_id: target.media, person_id: target.person ?? null };
  return null;
}

/** Modulo Persone a tre colonne: elenco · volti · scheda. */
export function PeopleModule() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const section = useLayout((s) => s.peopleSection);
  const setPeopleSection = useLayout((s) => s.setPeopleSection);
  const peopleTarget = useLayout((s) => s.peopleTarget);
  const clearPeopleTarget = useLayout((s) => s.clearPeopleTarget);
  const setModule = useLayout((s) => s.setModule);
  const setSelection = useLibrary((s) => s.setSelection);
  const [target, setTarget] = useState<GroupListTarget>(null);
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [group, setGroup] = useState<GroupRow | null>(null);
  const [identity, setIdentity] = useState<IdentityRow | null>(null);
  const [creating, setCreating] = useState(false);
  const [selectedFaceIds, setSelectedFaceIds] = useState<number[]>([]);
  const [arrival, setArrival] = useState<Arrival | null>(null);
  const [catalog, setCatalog] = useState<GroupRow[]>([]);
  const [multiMerge, setMultiMerge] = useState<GroupRow[] | null>(null);

  const groupListRef = useRef<GroupListHandle>(null);
  const faceGridRef = useRef<FaceGridHandle>(null);
  const personCardRef = useRef<PersonCardHandle>(null);
  const catalogRef = useRef(catalog);
  catalogRef.current = catalog;
  const targetRef = useRef(target);
  targetRef.current = target;
  const selectedIdsRef = useRef(selectedIds);
  selectedIdsRef.current = selectedIds;
  const groupRef = useRef(group);
  groupRef.current = group;
  const sectionRef = useRef(section);
  sectionRef.current = section;

  const mergeCandidates = useQuery({
    queryKey: ["people-groups", "review", "", "photos_desc", "merge-candidates"],
    queryFn: () => listAllGroups("review", "photos_desc"),
    enabled: isTauri() && section !== "people",
  });
  const peopleCandidates = useQuery({
    queryKey: ["people-groups", "people", "", "photos_desc", "merge-candidates"],
    queryFn: () => listAllGroups("people", "photos_desc"),
    enabled: isTauri() && section !== "people",
  });
  const candidates = (() => {
    const seen = new Set<number>();
    const out: GroupRow[] = [];
    for (const g of [...(mergeCandidates.data ?? []), ...(peopleCandidates.data ?? [])]) {
      if (seen.has(g.id)) continue;
      seen.add(g.id);
      out.push(g);
    }
    return out;
  })();

  const clearSelection = () => {
    setTarget(null);
    setSelectedIds([]);
    setGroup(null);
    setIdentity(null);
    setCreating(false);
    setSelectedFaceIds([]);
    setArrival(null);
  };

  const selectGroupRow = useCallback((g: GroupRow) => {
    setTarget({ kind: "group", id: g.id });
    setSelectedIds([g.id]);
    setGroup(g);
    setIdentity(null);
    setCreating(false);
    setSelectedFaceIds([]);
    setArrival(null);
    groupListRef.current?.scrollToGroup(g.id);
  }, []);

  /** Dopo aver nominato un gruppo in revisione, passa al successivo ancora in elenco. */
  const advanceAfterLink = useCallback(
    (linkedId: number) => {
      if (sectionRef.current !== "review") {
        setPeopleSection("people");
        setTarget({ kind: "group", id: linkedId });
        setSelectedIds([linkedId]);
        setGroup((g) =>
          g ? { ...g, id: linkedId, section: "people" } : stubGroup(linkedId, "people"),
        );
        setCreating(false);
        return;
      }
      const order = catalogRef.current.map((r) => r.id);
      const cur = targetRef.current?.kind === "group" ? targetRef.current.id : linkedId;
      const idx = order.indexOf(cur);
      const nextId =
        (idx >= 0 ? order.slice(idx + 1).find((id) => id !== linkedId) : undefined) ??
        order.find((id) => id !== linkedId && id !== cur);
      if (nextId == null) {
        clearSelection();
        return;
      }
      const next = catalogRef.current.find((r) => r.id === nextId);
      if (next) selectGroupRow(next);
      else clearSelection();
    },
    [selectGroupRow, setPeopleSection],
  );

  const onCatalogChange = useCallback((rows: GroupRow[]) => {
    setCatalog(rows);
    const t = targetRef.current;
    if (t?.kind !== "group") return;
    const updated = rows.find((r) => r.id === t.id);
    if (!updated) return;
    setGroup((prev) => {
      if (
        prev &&
        prev.id === updated.id &&
        prev.faces === updated.faces &&
        prev.photos === updated.photos &&
        prev.name === updated.name &&
        prev.mergeHint?.otherId === updated.mergeHint?.otherId
      ) {
        return prev;
      }
      return updated;
    });
  }, []);

  // faces://changed: invalida le query senza azzerare selezione / scorrimento.
  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    void events
      .onFacesChanged(() => {
        void Promise.all([
          qc.invalidateQueries({ queryKey: ["people-sections"] }),
          qc.invalidateQueries({ queryKey: ["people-groups"] }),
          qc.invalidateQueries({ queryKey: ["group-faces"] }),
          qc.invalidateQueries({ queryKey: ["people"] }),
          qc.invalidateQueries({ queryKey: ["identities"] }),
          qc.invalidateQueries({ queryKey: ["faces"] }),
        ]);
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => {
      unlisten?.();
    };
  }, [qc]);

  usePeopleHotkeys({
    navigateGroup(delta) {
      const order = catalogRef.current;
      if (!order.length) return;
      const cur =
        targetRef.current?.kind === "group" ? targetRef.current.id : selectedIdsRef.current[0];
      const idx = cur != null ? order.findIndex((g) => g.id === cur) : -1;
      const nextIdx =
        idx < 0
          ? delta > 0
            ? 0
            : order.length - 1
          : Math.max(0, Math.min(order.length - 1, idx + delta));
      const next = order[nextIdx];
      if (next) selectGroupRow(next);
    },
    toggleFocusedFace() {
      faceGridRef.current?.toggleFocusedSelection();
    },
    focusWhoIs() {
      personCardRef.current?.focusWhoIs();
    },
    focusMove() {
      faceGridRef.current?.focusMoveSelect();
    },
    notSame() {
      faceGridRef.current?.runNotSame();
    },
    merge() {
      const ids = selectedIdsRef.current;
      if (ids.length >= 2) {
        const rows = catalogRef.current.filter((g) => ids.includes(g.id));
        if (rows.length >= 2) {
          setMultiMerge(rows);
          return;
        }
      }
      if (!personCardRef.current?.openMergeIfReady()) {
        personCardRef.current?.focusMergeSelect();
      }
    },
    ignore() {
      if (personCardRef.current?.ignoreGroup()) return;
      const g = groupRef.current;
      if (!g || sectionRef.current === "ignored") return;
      void commands
        .correctPerson({ action: "ignore", group: g.id })
        .then(() =>
          Promise.all([
            qc.invalidateQueries({ queryKey: ["people-groups"] }),
            qc.invalidateQueries({ queryKey: ["people-sections"] }),
            qc.invalidateQueries({ queryKey: ["group-faces"] }),
          ]),
        );
    },
    undo() {
      void commands
        .undoPeople()
        .then(() =>
          Promise.all([
            qc.invalidateQueries({ queryKey: ["people-groups"] }),
            qc.invalidateQueries({ queryKey: ["people-sections"] }),
            qc.invalidateQueries({ queryKey: ["group-faces"] }),
            qc.invalidateQueries({ queryKey: ["people"] }),
            qc.invalidateQueries({ queryKey: ["identities"] }),
          ]),
        );
    },
  });

  useEffect(() => {
    if (!peopleTarget || !isTauri()) return;
    let cancelled = false;
    const run = async () => {
      const locateReq = toLocateRequest(peopleTarget);
      if (!locateReq) {
        // Solo `{ person }` lascia locateReq nullo (media/face usano locateFace).
        if (!("person" in peopleTarget) || "media" in peopleTarget || "face" in peopleTarget) {
          clearPeopleTarget();
          return;
        }
        const person = peopleTarget.person;
        setPeopleSection("people");
        setCreating(false);
        setIdentity(null);
        setArrival(null);
        setTarget({ kind: "group", id: person });
        setSelectedIds([person]);
        setSelectedFaceIds([]);
        const identities = await commands.listIdentities(null);
        if (cancelled) return;
        const row = identities.identities.find((r) => r.person === person) ?? null;
        setIdentity(row);
        setGroup(row ? stubGroup(person, "people", row.name) : await loadGroup("people", person));
        clearPeopleTarget();
        return;
      }

      const located = await commands.locateFace(locateReq);
      if (cancelled) return;
      setPeopleSection(located.section);
      setCreating(false);
      setIdentity(null);
      setSelectedFaceIds([]);
      if (located.singles) {
        setTarget({ kind: "singles" });
        setSelectedIds([]);
        setGroup(null);
      } else if (located.groupId == null && located.discarded) {
        // Volto scartato per qualità e senza gruppo: si apre l'elenco dei volti scartati.
        setTarget({ kind: "discarded" });
        setSelectedIds([]);
        setGroup(null);
      } else if (located.groupId != null) {
        setTarget({ kind: "group", id: located.groupId });
        setSelectedIds([located.groupId]);
        if (located.section === "people") {
          const identities = await commands.listIdentities(null);
          if (cancelled) return;
          const row = identities.identities.find((r) => r.person === located.groupId) ?? null;
          setIdentity(row);
          setGroup(
            row
              ? {
                  ...stubGroup(located.groupId, "people", row.name),
                  identityUid: row.uid,
                  photos: row.photos,
                }
              : await loadGroup("people", located.groupId),
          );
        } else {
          setGroup(await loadGroup(located.section, located.groupId));
        }
      } else {
        setTarget(null);
        setSelectedIds([]);
        setGroup(null);
      }
      setArrival({
        highlight: {
          faceId: located.faceId,
          index: located.index,
          discarded: located.discarded,
        },
        mediaId: located.mediaId,
        mediaName: located.mediaName,
      });
      clearPeopleTarget();
    };
    void run().catch(() => {
      if (!cancelled) clearPeopleTarget();
    });
    return () => {
      cancelled = true;
    };
  }, [peopleTarget, clearPeopleTarget, setPeopleSection]);

  return (
    <section
      aria-label={t("people.title")}
      style={{
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        display: "flex",
        flexWrap: "wrap",
        overflow: "hidden",
        position: "relative",
      }}
    >
      {arrival && (
        <div
          role="status"
          data-testid="people-arrival"
          style={{
            position: "absolute",
            top: 8,
            left: "50%",
            transform: "translateX(-50%)",
            zIndex: 5,
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "8px 12px",
            background: "var(--pc-raised)",
            border: "1px solid var(--pc-border)",
            borderRadius: 6,
            boxShadow: "0 4px 16px rgba(0,0,0,0.25)",
            maxWidth: "90%",
          }}
        >
          <span>{t("people.highlightedFrom", { name: arrival.mediaName })}</span>
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => {
              const mediaId = arrival.mediaId;
              setArrival(null);
              setSelection([mediaId], mediaId);
              setModule("library");
            }}
          >
            {t("people.backToPhoto")}
          </button>
        </div>
      )}
      <GroupList
        ref={groupListRef}
        section={section}
        onSectionChange={(s) => {
          setPeopleSection(s);
          clearSelection();
        }}
        target={target}
        selectedIds={selectedIds}
        onCatalogChange={onCatalogChange}
        onCreateIdentity={() => {
          setCreating(true);
          setIdentity(null);
          setGroup(null);
          setTarget(null);
          setSelectedIds([]);
          setArrival(null);
        }}
        onSelect={({ target: next, selectedIds: ids, group: g, identity: id }) => {
          setTarget(next);
          setSelectedIds(ids);
          setGroup(g);
          setIdentity(id);
          setCreating(false);
          setSelectedFaceIds([]);
          setArrival(null);
        }}
      />

      <main
        aria-label={t("people.facesLabel")}
        style={{
          flex: "999 1 420px",
          minWidth: 0,
          minHeight: 0,
          display: "flex",
          flexDirection: "column",
          borderRight: "1px solid var(--pc-border)",
          background: "var(--pc-bg)",
        }}
      >
        {target?.kind === "identity" ? (
          <div style={{ padding: 16 }}>
            <p className="muted" style={{ margin: 0 }}>
              {t("people.noPhotosInLibrary")}
            </p>
          </div>
        ) : (
          <FaceGrid
            ref={faceGridRef}
            target={target}
            group={group}
            highlight={arrival?.highlight ?? null}
            onSelectedFacesChange={setSelectedFaceIds}
          />
        )}
      </main>

      <aside
        aria-label={t("people.cardLabel")}
        style={{
          flex: "1 1 260px",
          maxWidth: 340,
          minWidth: 0,
          minHeight: 0,
          background: "var(--pc-panel)",
          padding: 16,
          display: "flex",
          flexDirection: "column",
          overflow: "auto",
          borderLeft: "1px solid var(--pc-border)",
        }}
      >
        <PersonCard
          ref={personCardRef}
          section={section}
          target={target}
          group={group}
          identity={identity}
          creating={creating}
          selectedFaceIds={selectedFaceIds}
          mergeCandidates={candidates}
          onLinked={advanceAfterLink}
          onIdentitySaved={(uid) => {
            setCreating(false);
            void commands.listIdentities(null).then((v) => {
              const row = v.identities.find((r) => r.uid === uid) ?? null;
              setIdentity(row);
              if (row?.person != null) {
                setTarget({ kind: "group", id: row.person });
                setSelectedIds([row.person]);
                setGroup({
                  ...stubGroup(row.person, "people", row.name),
                  identityUid: row.uid,
                  photos: row.photos,
                });
              } else {
                setTarget({ kind: "identity", uid });
                setSelectedIds([]);
                setGroup(null);
              }
            });
          }}
          onIdentityDeleted={clearSelection}
          onCreatingChange={setCreating}
        />
      </aside>

      {multiMerge && multiMerge.length >= 2 && (
        <MergeDialog
          sources={multiMerge.slice(1)}
          target={multiMerge[0]!}
          onClose={() => setMultiMerge(null)}
          onMerged={(to) => {
            setMultiMerge(null);
            void Promise.all([
              qc.invalidateQueries({ queryKey: ["people-groups"] }),
              qc.invalidateQueries({ queryKey: ["people-sections"] }),
              qc.invalidateQueries({ queryKey: ["group-faces"] }),
            ]).then(() => advanceAfterLink(to));
          }}
        />
      )}
    </section>
  );
}
