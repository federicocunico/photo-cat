import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type GroupRow,
  type GroupSort,
  type GroupTarget,
  type IdentityRow,
  type PeopleSection,
  type PeopleSections,
} from "../../lib/ipc";
import { QueryView } from "../../components/feedback/QueryView";
import { Button } from "../../components/ui";
import { useActiveGrid } from "../../commands/gridShortcuts";
import { notify, notifyError } from "../../store/toasts";
import { notifyPeopleUndo, refreshPeople, refreshPeopleFast } from "./peopleUndo";
import { PersonPicker, type PickerChoice } from "./PersonPicker";
import { withNewIdentity } from "./newIdentity";
import { PhotoCountFilter } from "./PhotoCountFilter";
import "./people.css";
import { fmtDate, fmtDec, fmtInt } from "../../lib/format";
import { useDebounced } from "../library/facets";
import { clickSelect } from "../../store/library";
import { faceUrl, thumbUrl } from "../library/thumbUrl";

const PAGE = 50;
const ROW_H = 58;
const LABEL_H = 28;

export type GroupListTarget =
  | { kind: "group"; id: number }
  | { kind: "identity"; uid: string }
  | { kind: "singles" }
  | { kind: "discarded" }
  | null;

export type GroupListSelection = {
  target: GroupListTarget;
  selectedIds: number[];
  anchor: number | null;
  group: GroupRow | null;
  identity: IdentityRow | null;
};

export type GroupListHandle = {
  scrollToGroup: (id: number) => void;
  scrollTop: () => number;
};

type VirtItem =
  | { kind: "label"; key: string; label: string }
  | { kind: "group"; group: GroupRow }
  | { kind: "singles"; count: number }
  | { kind: "discarded"; count: number };

const SORTS: GroupSort[] = ["photos_desc", "name_asc", "faces_desc"];

/** Gruppi al massimo in un'assegnazione in blocco (come `people::MAX_BULK_GROUPS`). */
const MAX_BULK_GROUPS = 10_000;

/** Riga di un'identità (come quella di un gruppo) più lo spazio tra le righe. */
const ID_ROW_H = ROW_H + 2;

/** In jsdom / flex annidato l'altezza misurata può essere 0: resta l'initialRect. */
function observeRect(el: HTMLElement | null, cb: (r: { width: number; height: number }) => void) {
  const emit = () => {
    const w = el?.clientWidth ?? 0;
    const h = el?.clientHeight ?? 0;
    cb({ width: w || 300, height: h || 600 });
  };
  emit();
  if (!el || typeof ResizeObserver === "undefined") return () => {};
  const ro = new ResizeObserver(emit);
  ro.observe(el);
  return () => ro.disconnect();
}

/** Dialogo aperto, con la selezione fissata all'apertura (la conferma riguarda proprio quella). */
type BulkDialog = { kind: "merge"; uids: string[] } | { kind: "assign"; ids: number[] } | null;

function Cover({ group }: { group: GroupRow }) {
  const [failed, setFailed] = useState(false);
  const cover = group.cover;
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  return (
    <span
      aria-hidden
      className="people-cover people-cover--sm"
      style={{
        backgroundImage: placeholder && !src ? `url("${placeholder}")` : undefined,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          className="people-cover-img"
        />
      )}
    </span>
  );
}

function groupTitle(g: GroupRow, t: (k: string, o?: Record<string, unknown>) => string) {
  if (g.name) return g.name;
  return t("people.unnamed", { count: g.photos });
}

function groupSub(g: GroupRow, t: (k: string, o?: Record<string, unknown>) => string) {
  if (g.section === "ignored") {
    const parts: string[] = [];
    if (g.ignoredAt != null) parts.push(t("people.ignoredOn", { date: fmtDate(g.ignoredAt) }));
    if (g.newFaces > 0) parts.push(t("people.newFaces", { count: g.newFaces }));
    return parts.join(" · ") || t("people.noSuggestion");
  }
  if (g.nameHint?.name) {
    const score = g.nameHint.score != null ? ` · ${fmtDec(g.nameHint.score, 2)}` : "";
    return t("people.maybeName", { name: g.nameHint.name }) + score;
  }
  if (g.mergeHint) return t("people.mergeHintSub");
  return t("people.noSuggestion");
}

export const GroupList = forwardRef<
  GroupListHandle,
  {
    section: PeopleSection;
    onSectionChange: (s: PeopleSection) => void;
    target: GroupListTarget;
    selectedIds: number[];
    onSelect: (next: GroupListSelection) => void;
    onCreateIdentity?: () => void;
    /** Cambia solo l'insieme dei gruppi selezionati (Seleziona tutto, Esc), senza aprirne uno. */
    onSelectedIdsChange?: (ids: number[]) => void;
    /** Elenco corrente dei gruppi (per tastiera e refresh senza perdere la selezione). */
    onCatalogChange?: (rows: GroupRow[]) => void;
  }
>(function GroupList(
  {
    section,
    onSectionChange,
    target,
    selectedIds,
    onSelect,
    onCreateIdentity,
    onSelectedIdsChange,
    onCatalogChange,
  },
  ref,
) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<GroupSort>("photos_desc");
  const [groupFilter, setGroupFilter] = useState<string | null>(null);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [restoreBusy, setRestoreBusy] = useState<number | null>(null);
  // Filtro "con al massimo N foto" e selezione multipla delle identità (sezione Persone).
  const [maxPhotos, setMaxPhotos] = useState<number | null>(null);
  const [idSel, setIdSel] = useState<string[]>([]);
  const [idAnchor, setIdAnchor] = useState<string | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [dialog, setDialog] = useState<BulkDialog>(null);
  const asideRef = useRef<HTMLElement>(null);
  // L'utente sta lavorando sull'elenco (ultimo clic o focus dentro): Ctrl+A va qui, non ai volti.
  const listActive = useRef(false);
  const debouncedQuery = useDebounced(query, 200);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const track = (e: Event) => {
      const node = e.target instanceof Node ? e.target : null;
      // I dialoghi (in un portale) non cambiano la vista su cui si sta lavorando.
      if (node instanceof Element && node.closest('[role="dialog"]')) return;
      listActive.current = node != null && (asideRef.current?.contains(node) ?? false);
    };
    document.addEventListener("pointerdown", track, true);
    document.addEventListener("focusin", track, true);
    return () => {
      document.removeEventListener("pointerdown", track, true);
      document.removeEventListener("focusin", track, true);
    };
  }, []);

  const restoreGroup = async (id: number, e: MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    setRestoreBusy(id);
    try {
      await commands.correctPerson({ action: "restore", group: id });
      await refreshPeople(qc);
      notifyPeopleUndo(qc, t("people.restoredDone"));
    } catch (err) {
      notifyError(err);
    } finally {
      setRestoreBusy(null);
    }
  };

  const sections = useQuery({
    queryKey: ["people-sections"],
    queryFn: commands.peopleSections,
    enabled: isTauri(),
  });

  const identities = useQuery({
    queryKey: ["identities", debouncedQuery.trim(), maxPhotos],
    queryFn: () => commands.listIdentities(debouncedQuery.trim() || null, maxPhotos),
    enabled: isTauri() && section === "people",
  });

  const groups = useInfiniteQuery({
    queryKey: ["people-groups", section, debouncedQuery, sort, maxPhotos],
    queryFn: ({ pageParam }) =>
      commands.listGroups(section, debouncedQuery || null, sort, pageParam, PAGE, maxPhotos),
    initialPageParam: null as number | null,
    getNextPageParam: (last) => last.next ?? undefined,
    enabled: isTauri() && section !== "people",
    placeholderData: (prev) => prev,
  });

  const rows = useMemo(() => groups.data?.pages.flatMap((p) => p.items) ?? [], [groups.data]);

  const identityRows = useMemo(() => {
    const all = identities.data?.identities ?? [];
    return groupFilter === null ? all : all.filter((r) => r.group === groupFilter);
  }, [identities.data, groupFilter]);

  const counts: PeopleSections = sections.data ?? {
    review: 0,
    people: 0,
    ignored: 0,
    singles: 0,
    discarded: 0,
  };
  const peopleCount =
    section === "people" ? (identities.data?.identities.length ?? counts.people) : counts.people;

  const virtItems = useMemo((): VirtItem[] => {
    if (section === "people") return [];
    const suggested = rows.filter((g) => g.nameHint || g.mergeHint);
    const others = rows.filter((g) => !g.nameHint && !g.mergeHint);
    const out: VirtItem[] = [];
    if (suggested.length) {
      out.push({ kind: "label", key: "suggested", label: t("people.suggestedForName") });
      for (const g of suggested) out.push({ kind: "group", group: g });
    }
    if (others.length) {
      if (suggested.length)
        out.push({ kind: "label", key: "others", label: t("people.otherGroups") });
      for (const g of others) out.push({ kind: "group", group: g });
    }
    if (section === "review" && counts.singles > 0) {
      out.push({ kind: "label", key: "singles", label: t("people.singles") });
      out.push({ kind: "singles", count: counts.singles });
    }
    // Volti scartati per qualità senza gruppo: altrimenti non sarebbero raggiungibili.
    if (section === "review" && counts.discarded > 0) {
      out.push({ kind: "label", key: "discarded", label: t("people.discardedAllTitle") });
      out.push({ kind: "discarded", count: counts.discarded });
    }
    return out;
  }, [rows, section, counts.singles, counts.discarded, t]);

  const order = useMemo(
    () =>
      virtItems
        .filter((i): i is Extract<VirtItem, { kind: "group" }> => i.kind === "group")
        .map((i) => i.group.id),
    [virtItems],
  );

  const catalogKey = rows.map((r) => `${r.id}:${r.faces}:${r.photos}:${r.name ?? ""}`).join(",");
  useEffect(() => {
    onCatalogChange?.(rows);
    // catalogKey evita di notificare a ogni render del padre con lo stesso elenco.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- rows segue catalogKey
  }, [catalogKey, onCatalogChange]);

  const virt = useVirtualizer({
    count: virtItems.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (virtItems[i]?.kind === "label" ? LABEL_H : ROW_H),
    overscan: 8,
    initialRect: { width: 300, height: 600 },
    observeElementRect: (_instance, cb) => observeRect(scrollRef.current, cb),
  });

  // Anche le identità sono virtualizzate: con migliaia di righe e la selezione multipla, ogni
  // clic ridisegnerebbe tutto l'elenco.
  const idVirt = useVirtualizer({
    count: section === "people" ? identityRows.length : 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ID_ROW_H,
    overscan: 8,
    initialRect: { width: 300, height: 600 },
    observeElementRect: (_instance, cb) => observeRect(scrollRef.current, cb),
  });

  useImperativeHandle(ref, () => ({
    scrollToGroup(id: number) {
      if (section === "people") {
        const index = identityRows.findIndex((r) => r.person === id);
        if (index >= 0) idVirt.scrollToIndex(index, { align: "auto" });
        return;
      }
      const index = virtItems.findIndex((i) => i.kind === "group" && i.group.id === id);
      if (index >= 0) virt.scrollToIndex(index, { align: "auto" });
    },
    scrollTop: () => scrollRef.current?.scrollTop ?? 0,
  }));

  const vitems = virt.getVirtualItems();
  const last = vitems[vitems.length - 1]?.index ?? -1;
  useEffect(() => {
    if (last >= virtItems.length - 3 && groups.hasNextPage && !groups.isFetchingNextPage)
      void groups.fetchNextPage();
  }, [last, virtItems.length, groups]);

  const tab = (id: PeopleSection, label: string, count: number) => (
    <button
      key={id}
      type="button"
      role="tab"
      aria-selected={section === id}
      className="btn people-tab"
      onClick={() => onSectionChange(id)}
      style={{
        background: section === id ? "var(--pc-raised)" : "transparent",
        borderColor: section === id ? "var(--pc-border)" : "transparent",
      }}
    >
      <b className="people-tab-label">{label}</b>
      <span className="mono muted people-muted-sm">
        {id === "review" ? t("people.groupsCount", { count }) : fmtInt(count)}
      </span>
    </button>
  );

  const selectGroup = (id: number, e: MouseEvent, group: GroupRow) => {
    const next = clickSelect(order, selectedIds, anchor, id, {
      ctrl: e.ctrlKey || e.metaKey,
      shift: e.shiftKey,
    });
    setAnchor(id);
    onSelect({
      target: { kind: "group", id },
      selectedIds: next,
      anchor: id,
      group,
      identity: null,
    });
  };

  const selectIdentity = (row: IdentityRow) => {
    onSelect({
      target:
        row.person != null ? { kind: "group", id: row.person } : { kind: "identity", uid: row.uid },
      selectedIds: row.person != null ? [row.person] : [],
      anchor: row.person,
      group:
        row.person != null
          ? {
              id: row.person,
              name: row.name,
              identityUid: row.uid,
              section: "people",
              faces: 0,
              goodFaces: 0,
              discardedFaces: 0,
              photos: row.photos,
              cover: null,
              ignoredAt: null,
              newFaces: 0,
              nameHint: null,
              mergeHint: null,
            }
          : null,
      identity: row,
    });
  };

  // --- Selezione multipla e azioni in blocco (Persone e Da rivedere) ---------------------------
  useEffect(() => {
    setMaxPhotos(null);
    setIdSel([]);
    setIdAnchor(null);
    setDialog(null);
  }, [section]);

  /** Solo le identità che si vedono: un'unione non annullabile non tocca righe nascoste. */
  const selectedUids = useMemo(() => {
    const set = new Set(idSel);
    return identityRows.filter((r) => set.has(r.uid)).map((r) => r.uid);
  }, [idSel, identityRows]);
  const selectedUidSet = useMemo(() => new Set(selectedUids), [selectedUids]);
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const bulkSection = section === "people" || section === "review";
  const selectedCount = section === "people" ? selectedUids.length : selectedIds.length;

  const clickIdentity = (row: IdentityRow, e: MouseEvent) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl && !e.shiftKey) {
      setIdSel([row.uid]);
      setIdAnchor(row.uid);
      selectIdentity(row);
      return;
    }
    const order = identityRows.map((r) => r.uid);
    const a = idAnchor != null ? order.indexOf(idAnchor) : -1;
    const b = order.indexOf(row.uid);
    if (e.shiftKey && a >= 0 && b >= 0) {
      const range = order.slice(Math.min(a, b), Math.max(a, b) + 1);
      setIdSel(ctrl ? Array.from(new Set([...selectedUids, ...range])) : range);
      return;
    }
    setIdSel(
      selectedUidSet.has(row.uid)
        ? selectedUids.filter((u) => u !== row.uid)
        : [...selectedUids, row.uid],
    );
    setIdAnchor(row.uid);
  };

  const resolveAllIds = async (): Promise<number[] | string[]> =>
    section === "people"
      ? groupFilter === null
        ? commands.identitySelectionUids(debouncedQuery.trim() || null, maxPhotos)
        : identityRows.map((r) => r.uid)
      : commands.groupSelectionIds(section, debouncedQuery || null, maxPhotos);

  const applySelection = (all: Array<number | string>) => {
    if (section === "people") setIdSel(all as string[]);
    else onSelectedIdsChange?.(all as number[]);
  };

  const selectAll = async () => {
    setSelecting(true);
    try {
      applySelection(await resolveAllIds());
    } catch (e) {
      notifyError(e);
    } finally {
      setSelecting(false);
    }
  };
  const invertSelection = async () => {
    setSelecting(true);
    try {
      const all = await resolveAllIds();
      const have = new Set<number | string>(section === "people" ? selectedUids : selectedIds);
      applySelection(all.filter((x: number | string) => !have.has(x)));
    } catch (e) {
      notifyError(e);
    } finally {
      setSelecting(false);
    }
  };
  const clearSelection = () => {
    if (section === "people") {
      setIdSel([]);
      setIdAnchor(null);
    } else onSelectedIdsChange?.([]);
  };

  /**
   * Cambiando il filtro per numero di foto la selezione si azzera: non resta selezionato nulla
   * che il nuovo elenco non mostra (e "Seleziona tutto" riparte dal nuovo elenco).
   */
  const changeMaxPhotos = (max: number | null) => {
    if (max === maxPhotos) return;
    setMaxPhotos(max);
    clearSelection();
  };

  // Ctrl+A, Ctrl+Maiusc+I ed Esc agiscono sull'elenco mentre l'utente ci lavora (ultimo clic o
  // focus nell'elenco); altrimenti restano ai volti.
  useActiveGrid(
    {
      selectAll,
      invert: invertSelection,
      clear: clearSelection,
      count: () => selectedCount,
      hasFocus: () => listActive.current && (asideRef.current?.isConnected ?? false),
    },
    bulkSection,
  );

  const nameOf = (c: PickerChoice) => c.name;

  /** "Unisci in…": anteprima con i conteggi, poi unione in un'unica operazione. */
  const mergeProps = (uids: string[]) => ({
    mode: "identities" as const,
    title: t("people.bulk.mergePickTitle"),
    intro: t("people.bulk.mergePickIntro", { count: uids.length }),
    danger: true,
    describe: async (c: PickerChoice) => {
      if (c.kind !== "identity") throw new Error(t("people.failed"));
      const p = await commands.mergeIdentitiesPreview(uids, c.uid);
      if (p.identities === 0) throw new Error(t("people.bulk.mergeNothing", { name: c.name }));
      const more = p.identities - p.names.length;
      return {
        title: t("people.bulk.mergeTitle", { count: p.identities, name: c.name }),
        body: t("people.bulk.mergeBody", {
          name: c.name,
          photos: t("people.bulk.photosCount", { count: p.photos }),
          refs: t("people.bulk.refFacesCount", { count: p.referenceFaces }),
        }),
        details: p.names.length
          ? [
              more > 0
                ? t("people.bulk.mergeNamesMore", {
                    names: p.names.join(", "),
                    count: more,
                  })
                : t("people.bulk.mergeNames", { names: p.names.join(", ") }),
            ]
          : undefined,
        warning:
          p.withoutPhotos > 0
            ? t("people.bulk.mergeWithoutPhotos", { count: p.withoutPhotos })
            : undefined,
      };
    },
    confirmLabel: (c: PickerChoice) => t("people.bulk.mergeConfirm", { name: nameOf(c) }),
    run: async (c: PickerChoice) => {
      if (c.kind !== "identity") return;
      const n = await commands.mergeIdentitiesMany(uids, c.uid);
      notify({
        key: "people-bulk",
        tone: "ok",
        message: t("people.bulk.mergedDone", { count: n, name: c.name }),
      });
      setIdSel([]);
      setIdAnchor(null);
      onSelect({ target: null, selectedIds: [], anchor: null, group: null, identity: null });
      await refreshPeople(qc);
    },
  });

  /** "Assegna a…": tutti i volti dei gruppi selezionati in una persona, in un solo passaggio. */
  const assignProps = (ids: number[]) => ({
    mode: "assign" as const,
    title: t("people.bulk.assignPickTitle"),
    intro: t("people.bulk.assignPickIntro", { count: ids.length }),
    allowCreate: true,
    describe: (c: PickerChoice) => {
      // Lo stesso limite del backend: meglio dirlo qui che dopo la conferma.
      if (ids.length > MAX_BULK_GROUPS)
        throw new Error(t("people.bulk.tooMany", { max: fmtInt(MAX_BULK_GROUPS) }));
      return {
        title: t("people.bulk.assignTitle", { count: ids.length, name: c.name }),
        body: t("people.bulk.assignBody", { name: c.name }),
      };
    },
    confirmLabel: (c: PickerChoice) => t("people.bulk.assignConfirm", { name: nameOf(c) }),
    run: async (c: PickerChoice) => {
      const assign = (to: GroupTarget) => commands.assignGroups(ids, to);
      const out =
        c.kind === "person"
          ? await assign({ kind: "person", person: c.person })
          : c.kind === "identity"
            ? await assign({ kind: "identity", uid: c.uid })
            : await withNewIdentity(c.name, (uid) => assign({ kind: "identity", uid }));
      let message = t("people.bulk.assignedDone", {
        count: out.groups,
        faces: t("people.bulk.facesCount", { count: out.faces }),
        name: c.name,
      });
      if (out.skippedFaces > 0)
        message += " " + t("people.bulk.skipped", { count: out.skippedFaces });
      notifyPeopleUndo(qc, message);
      onSelect({ target: null, selectedIds: [], anchor: null, group: null, identity: null });
      await refreshPeopleFast(qc);
    },
  });

  const emptyMsg =
    section === "review"
      ? t("people.emptyReview")
      : section === "people"
        ? t("people.emptyPeople")
        : t("people.emptyIgnored");

  const idGroups = identities.data?.groups ?? [];
  const everyone = identities.data?.identities ?? [];

  return (
    <aside ref={asideRef} aria-label={t("people.listLabel")} className="people-list">
      <div className="people-list-head">
        <div role="tablist" className="people-tabs">
          {tab("review", t("people.sectionReview"), counts.review)}
          {tab("people", t("people.sectionPeople"), peopleCount)}
          {tab("ignored", t("people.sectionIgnored"), counts.ignored)}
        </div>
        {section === "people" ? (
          <>
            <button className="btn btn-primary people-stretch" onClick={() => onCreateIdentity?.()}>
              {t("identities.newButton")}
            </button>
            <div className="people-wrap people-wrap--gap-sm">
              <button
                type="button"
                className="pill"
                aria-pressed={groupFilter === null}
                onClick={() => setGroupFilter(null)}
                style={{ borderColor: groupFilter === null ? "var(--pc-accent)" : undefined }}
              >
                {t("identities.allGroups", { count: everyone.length })}
              </button>
              {idGroups.map((g) => (
                <button
                  key={g}
                  type="button"
                  className="pill"
                  aria-pressed={groupFilter === g}
                  onClick={() => setGroupFilter(g)}
                  style={{ borderColor: groupFilter === g ? "var(--pc-accent)" : undefined }}
                >
                  {g} ({everyone.filter((r) => r.group === g).length})
                </button>
              ))}
            </div>
            <input
              type="search"
              className="field people-full"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("identities.search")}
              aria-label={t("identities.search")}
            />
          </>
        ) : (
          <>
            <input
              className="field people-full"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("people.filterPlaceholder")}
              aria-label={t("people.filterPlaceholder")}
            />
            <div className="people-wrap people-wrap--gap-sm">
              {SORTS.map((s) => (
                <button
                  key={s}
                  type="button"
                  className="chip people-sort-chip"
                  aria-pressed={sort === s}
                  onClick={() => setSort(s)}
                  style={{
                    border: sort === s ? "1px solid var(--pc-accent)" : "1px solid transparent",
                    background: sort === s ? "var(--pc-accent-soft-bg)" : "var(--pc-raised)",
                    color: sort === s ? "var(--pc-accent-soft-text)" : "var(--pc-text-soft)",
                  }}
                >
                  {t(`people.sort.${s}`)}
                </button>
              ))}
            </div>
          </>
        )}
        {bulkSection && (
          <div role="toolbar" aria-label={t("people.bulk.toolbar")} className="people-bulk-bar">
            <PhotoCountFilter
              key={section}
              value={maxPhotos}
              onChange={changeMaxPhotos}
              atMost={section === "people"}
            />
            <div className="people-wrap people-wrap--gap-sm">
              <Button size="sm" disabled={selecting} onClick={() => void selectAll()}>
                {t("people.selectAll")}
              </Button>
              <Button size="sm" disabled={selectedCount === 0} onClick={clearSelection}>
                {t("people.bulk.clear")}
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabledReason={
                  selectedCount === 0
                    ? t(section === "people" ? "people.bulk.noIdentities" : "people.bulk.noGroups")
                    : undefined
                }
                onClick={() =>
                  setDialog(
                    section === "people"
                      ? { kind: "merge", uids: selectedUids }
                      : { kind: "assign", ids: selectedIds },
                  )
                }
              >
                {section === "people"
                  ? t("people.bulk.mergeAction")
                  : t("people.bulk.assignAction")}
              </Button>
            </div>
            {/* Sempre presente: il cambio di conteggio (Ctrl+A, Esc) viene annunciato. */}
            <span aria-live="polite" aria-atomic="true" className="muted people-muted-sm">
              {selectedCount > 0 ? t("people.bulk.selectedCount", { count: selectedCount }) : null}
            </span>
          </div>
        )}
      </div>

      <div ref={scrollRef} data-testid="group-list-scroll" className="people-list-scroll">
        {section === "people" ? (
          <QueryView
            query={identities}
            isEmpty={() => identityRows.length === 0}
            error={t("people.failed")}
            empty={<p className="muted">{emptyMsg}</p>}
          >
            {() => (
              <>
                <div
                  role="list"
                  aria-label={t("identities.title")}
                  className="people-virt"
                  style={{ height: idVirt.getTotalSize() }}
                >
                  {idVirt.getVirtualItems().map((vi) => {
                    const r = identityRows[vi.index];
                    if (!r) return null;
                    const checked = selectedUidSet.has(r.uid);
                    const current =
                      (target?.kind === "identity" && target.uid === r.uid) ||
                      (target?.kind === "group" && r.person != null && target.id === r.person);
                    const selected = checked || current;
                    return (
                      <div
                        key={r.uid}
                        role="listitem"
                        className="people-virt-row"
                        style={{ height: ROW_H, transform: `translateY(${vi.start}px)` }}
                      >
                        <button
                          type="button"
                          aria-current={current || undefined}
                          aria-pressed={checked}
                          onClick={(e) => clickIdentity(r, e)}
                          className="people-list-item people-list-item--id"
                          style={{
                            border: selected
                              ? "1px solid var(--pc-selection-border)"
                              : "1px solid transparent",
                            background: selected ? "var(--pc-selection-bg)" : "transparent",
                          }}
                        >
                          <span aria-hidden className="people-initials">
                            {r.name
                              .split(/\s+/)
                              .filter(Boolean)
                              .slice(0, 2)
                              .map((w) => w[0]?.toUpperCase())
                              .join("")}
                          </span>
                          <span className="people-list-text">
                            <span className="people-list-name">{r.name}</span>
                            <span className="muted people-muted-sm">
                              {r.group ?? t("identities.notLinkedShort")}
                              {r.faces
                                ? ` · ${t("identities.refCount", { count: r.faces })}`
                                : ` · ${t("identities.noRefs")}`}
                            </span>
                          </span>
                          <span className="mono muted people-muted-sm">
                            {r.person != null && r.photos > 0
                              ? t("people.photos", { count: r.photos })
                              : t("people.noPhotosInLibrary")}
                          </span>
                        </button>
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </QueryView>
        ) : (
          <QueryView
            query={groups}
            isEmpty={() => rows.length === 0 && counts.singles === 0 && counts.discarded === 0}
            error={t("people.failed")}
            empty={<p className="muted">{emptyMsg}</p>}
          >
            {() => (
              <>
                <div
                  role="listbox"
                  aria-label={t("people.listLabel")}
                  aria-multiselectable
                  className="people-virt"
                  style={{ height: virt.getTotalSize() }}
                >
                  {vitems.map((vi) => {
                    const item = virtItems[vi.index];
                    if (!item) return null;
                    const groupSelected =
                      item.kind === "group" &&
                      ((target?.kind === "group" && target.id === item.group.id) ||
                        selectedIdSet.has(item.group.id));
                    return (
                      <div
                        key={vi.key}
                        className="people-virt-row"
                        style={{
                          height: vi.size,
                          transform: `translateY(${vi.start}px)`,
                        }}
                      >
                        {item.kind === "label" && (
                          <div role="presentation" className="label people-section-label">
                            {item.label}
                          </div>
                        )}
                        {item.kind === "group" && (
                          <div className={section === "ignored" ? "people-row" : undefined}>
                            <div
                              role="option"
                              tabIndex={0}
                              data-group-id={item.group.id}
                              aria-selected={
                                target?.kind === "group" && target.id === item.group.id
                                  ? true
                                  : selectedIdSet.has(item.group.id)
                              }
                              onClick={(e) => selectGroup(item.group.id, e, item.group)}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                  e.preventDefault();
                                  selectGroup(
                                    item.group.id,
                                    e as unknown as MouseEvent,
                                    item.group,
                                  );
                                }
                              }}
                              className={
                                section === "ignored"
                                  ? "people-list-item people-list-item--ignored"
                                  : "people-list-item people-list-item--group"
                              }
                              style={{
                                border: groupSelected
                                  ? "1px solid var(--pc-selection-border)"
                                  : "1px solid transparent",
                                background: groupSelected
                                  ? "var(--pc-selection-bg)"
                                  : "transparent",
                              }}
                            >
                              <Cover group={item.group} />
                              <span className="people-list-text">
                                <span className="people-list-name">
                                  {groupTitle(item.group, t)}
                                </span>
                                <span className="muted people-muted-sm people-list-sub">
                                  {groupSub(item.group, t)}
                                </span>
                              </span>
                              {section !== "ignored" && (
                                <span className="mono muted people-muted-sm">
                                  {t("people.photos", { count: item.group.photos })}
                                </span>
                              )}
                            </div>
                            {/* "Ripristina" accanto alla riga, non dentro: niente controlli annidati. */}
                            {section === "ignored" && (
                              <Button
                                size="sm"
                                disabled={restoreBusy === item.group.id}
                                onClick={(e) => restoreGroup(item.group.id, e)}
                              >
                                {t("people.restore")}
                              </Button>
                            )}
                          </div>
                        )}
                        {item.kind === "singles" && (
                          <button
                            type="button"
                            onClick={() =>
                              onSelect({
                                target: { kind: "singles" },
                                selectedIds: [],
                                anchor: null,
                                group: null,
                                identity: null,
                              })
                            }
                            role="option"
                            aria-selected={target?.kind === "singles"}
                            className="people-list-item people-list-item--pair"
                            style={{
                              border:
                                target?.kind === "singles"
                                  ? "1px solid var(--pc-selection-border)"
                                  : "1px solid transparent",
                              background:
                                target?.kind === "singles"
                                  ? "var(--pc-selection-bg)"
                                  : "transparent",
                            }}
                          >
                            <span aria-hidden className="people-cover-ph" />
                            <span className="people-list-text">
                              <span className="people-list-name-plain">
                                {t("people.singlesLabel", { count: item.count })}
                              </span>
                              <span className="muted people-muted-sm">
                                {t("people.singlesSub")}
                              </span>
                            </span>
                          </button>
                        )}
                        {item.kind === "discarded" && (
                          <button
                            type="button"
                            onClick={() =>
                              onSelect({
                                target: { kind: "discarded" },
                                selectedIds: [],
                                anchor: null,
                                group: null,
                                identity: null,
                              })
                            }
                            role="option"
                            aria-selected={target?.kind === "discarded"}
                            className="people-list-item people-list-item--pair"
                            style={{
                              border:
                                target?.kind === "discarded"
                                  ? "1px solid var(--pc-selection-border)"
                                  : "1px solid transparent",
                              background:
                                target?.kind === "discarded"
                                  ? "var(--pc-selection-bg)"
                                  : "transparent",
                            }}
                          >
                            <span aria-hidden className="people-cover-ph" />
                            <span className="people-list-text">
                              <span className="people-list-name-plain">
                                {t("people.discardedAllLabel", { count: item.count })}
                              </span>
                              <span className="muted people-muted-sm">
                                {t("people.discardedAllSub")}
                              </span>
                            </span>
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </QueryView>
        )}
      </div>

      {dialog?.kind === "merge" && (
        <PersonPicker {...mergeProps(dialog.uids)} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "assign" && (
        <PersonPicker {...assignProps(dialog.ids)} onClose={() => setDialog(null)} />
      )}
    </aside>
  );
});
