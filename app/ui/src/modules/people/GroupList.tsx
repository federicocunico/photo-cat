import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
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
import { Button, ContextMenu, type MenuItem } from "../../components/ui";
import { useActivePane } from "../../commands/gridShortcuts";
import { notify, notifyError } from "../../store/toasts";
import { notifyPeopleUndo, refreshPeople, refreshPeopleFast } from "./peopleUndo";
import { PersonPicker, type PickerChoice } from "./PersonPicker";
import { withNewIdentity } from "./newIdentity";
import { PhotoCountFilter } from "./PhotoCountFilter";
import "./people.css";
import { fmtDate, fmtDec, fmtInt } from "../../lib/format";
import { useDebounced } from "../library/facets";
import { idsSelection, selIds, selectForContextMenu } from "../../lib/selection/model";
import { useSelection, type SelectionStore } from "../../lib/selection/useSelection";
import { handleSelectionKey } from "../../lib/selection/useSelectionKeys";
import { faceUrl, thumbUrl } from "../library/thumbUrl";

const PAGE = 50;
const ROW_H = 58;
const LABEL_H = 28;

/** Id DOM delle voci: bersagli di `aria-activedescendant` degli elenchi. */
const groupOptionId = (id: number) => `group-opt-${id}`;
const identityOptionId = (uid: string) => `identity-opt-${uid}`;

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
  const [restoreBusy, setRestoreBusy] = useState<number | null>(null);
  // Filtro "con al massimo N foto" (Da rivedere e Persone).
  const [maxPhotos, setMaxPhotos] = useState<number | null>(null);
  // Focus della tastiera nell'elenco (distinto dalla selezione e dall'elemento aperto).
  // Il cursore dei gruppi vale finché non cambia il gruppo aperto: quando il modulo ne apre un
  // altro (avanzamento dopo un nome, apertura da un altro modulo) il focus torna su quello aperto.
  const openedGroupId = target?.kind === "group" ? target.id : null;
  const [groupCursorState, setGroupCursorState] = useState<{
    id: number;
    opened: number | null;
  } | null>(null);
  const setGroupCursor = (id: number) => setGroupCursorState({ id, opened: openedGroupId });
  const groupCursor =
    groupCursorState && groupCursorState.opened === openedGroupId ? groupCursorState.id : null;
  const [idCursor, setIdCursor] = useState<string | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [dialog, setDialog] = useState<BulkDialog>(null);
  const asideRef = useRef<HTMLElement>(null);
  const debouncedQuery = useDebounced(query, 200);
  const scrollRef = useRef<HTMLDivElement>(null);

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

  // Gruppi nell'ordine mostrato (suggeriti, poi altri gruppi): lo seguono frecce e avanzamento.
  const shownRows = useMemo(
    () =>
      virtItems
        .filter((i): i is Extract<VirtItem, { kind: "group" }> => i.kind === "group")
        .map((i) => i.group),
    [virtItems],
  );
  const order = useMemo(() => shownRows.map((g) => g.id), [shownRows]);

  const catalogKey = shownRows
    .map((r) => `${r.id}:${r.faces}:${r.photos}:${r.name ?? ""}`)
    .join(",");
  useEffect(() => {
    onCatalogChange?.(shownRows);
    // catalogKey evita di notificare a ogni render del padre con lo stesso elenco.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- shownRows segue catalogKey
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
    >
      <b className="people-tab-label">{label}</b>
      <span className="muted people-muted-sm">
        {id === "review" ? t("people.groupsCount", { count }) : fmtInt(count)}
      </span>
    </button>
  );

  /** Apre un gruppo nei riquadri dei volti e della scheda, con la selezione corrente. */
  const openGroup = (id: number, group: GroupRow) => {
    setGroupCursor(id);
    onSelect({
      target: { kind: "group", id },
      selectedIds: selectedRef.current,
      anchor: anchorRef.current,
      group,
      identity: null,
    });
  };
  // Da tastiera l'apertura aspetta 150 ms di quiete: tenendo premuta una freccia si scorre
  // l'elenco senza caricare i volti di ogni gruppo attraversato.
  const openTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const openSoon = (open: () => void) => {
    if (openTimer.current) clearTimeout(openTimer.current);
    openTimer.current = setTimeout(() => {
      openTimer.current = null;
      open();
    }, 150);
  };
  useEffect(
    () => () => {
      if (openTimer.current) clearTimeout(openTimer.current);
    },
    [],
  );
  const selectGroup = (id: number, e: MouseEvent, group: GroupRow) => {
    gsel.click(id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
    openGroup(id, group);
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

  // --- Selezione (modello comune, unified-selection) e azioni in blocco ------------------------
  // Gruppi: la selezione vive nel modulo (`selectedIds`), qui l'ancora; identità: stato locale.
  const selectedRef = useRef(selectedIds);
  selectedRef.current = selectedIds;
  const anchorRef = useRef<number | null>(null);
  const groupStore: SelectionStore<null, number> = {
    selection: idsSelection<null, number>(selectedIds),
    getSelection: () => idsSelection<null, number>(selectedRef.current),
    getAnchor: () => anchorRef.current,
    setAnchor: (a) => {
      anchorRef.current = a;
    },
    replace: (s) => {
      const ids = selIds(s) ?? [];
      selectedRef.current = ids;
      onSelectedIdsChange?.(ids);
    },
  };
  const gsel = useSelection<null, number>({
    filter: null,
    order,
    fetchAll: () => commands.groupSelectionIds(section, debouncedQuery || null, maxPhotos),
    state: groupStore,
    onError: notifyError,
  });
  const identityUids = useMemo(() => identityRows.map((r) => r.uid), [identityRows]);
  const isel = useSelection<null, string>({
    filter: null,
    order: identityUids,
    fetchAll: () =>
      groupFilter === null
        ? commands.identitySelectionUids(debouncedQuery.trim() || null, maxPhotos)
        : Promise.resolve(identityUids),
    onError: notifyError,
  });

  useEffect(() => {
    setMaxPhotos(null);
    isel.clear();
    anchorRef.current = null;
    setDialog(null);
  }, [section]); // eslint-disable-line react-hooks/exhaustive-deps

  /** Solo le identità che si vedono: un'unione non annullabile non tocca righe nascoste. */
  const selectedUids = useMemo(
    () => identityRows.filter((r) => isel.has(r.uid)).map((r) => r.uid),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- isel.has segue la selezione
    [isel.selection, identityRows],
  );
  const selectedUidSet = useMemo(() => new Set(selectedUids), [selectedUids]);
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const bulkSection = section === "people" || section === "review";
  const selectedCount = section === "people" ? selectedUids.length : selectedIds.length;

  const clickIdentity = (row: IdentityRow, e: MouseEvent) => {
    const mods = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
    isel.click(row.uid, mods);
    setIdCursor(row.uid);
    // Un clic semplice seleziona solo quella persona e la apre.
    if (!mods.ctrl && !mods.shift) selectIdentity(row);
  };

  const selectAll = async () => {
    setSelecting(true);
    try {
      await (section === "people" ? isel.selectAll() : gsel.selectAll());
    } finally {
      setSelecting(false);
    }
  };
  const invertSelection = async () => {
    setSelecting(true);
    try {
      await (section === "people" ? isel.invert() : gsel.invert());
    } finally {
      setSelecting(false);
    }
  };
  const clearSelection = () => {
    if (section === "people") isel.clear();
    else gsel.clear();
  };

  // Tastiera nell'elenco (modello di selezione comune): il focus resta sull'elenco, le frecce
  // selezionano e aprono l'elemento raggiunto (dopo 150 ms), Maiusc estende, Ctrl sposta solo il
  // focus, Ctrl+Spazio aggiunge o toglie, Invio e Spazio aprono l'elemento con il focus.
  const listPage = () => Math.max(1, Math.floor((scrollRef.current?.clientHeight ?? 600) / ROW_H));
  const groupRows = useMemo(() => new Map(rows.map((g) => [g.id, g])), [rows]);
  const groupCursorId = groupCursor ?? (target?.kind === "group" ? target.id : null);
  const groupCursorIndex = groupCursorId == null ? -1 : order.indexOf(groupCursorId);
  const onGroupsKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    const mod = e.ctrlKey || e.metaKey;
    const plain = !mod && !e.shiftKey && !e.altKey;
    if (plain && (e.key === "Enter" || e.key === " ") && groupCursorId != null) {
      const g = groupRows.get(groupCursorId);
      if (!g) return;
      e.preventDefault();
      if (e.key === " ") gsel.click(groupCursorId, { ctrl: false, shift: false });
      openGroup(groupCursorId, g);
      return;
    }
    handleSelectionKey(e, {
      layout: "list",
      pageSize: listPage(),
      count: order.length,
      cursor: groupCursorIndex,
      idAt: (i) => order[i],
      moveTo: (i) => {
        const id = order[i];
        if (id == null) return;
        setGroupCursor(id);
        const vi = virtItems.findIndex((it) => it.kind === "group" && it.group.id === id);
        if (vi >= 0) virt.scrollToIndex(vi, { align: "auto" });
      },
      select: (id, mods) => {
        gsel.click(id, mods);
        if (mods.ctrl && !mods.shift) return; // Ctrl+Spazio: cambia solo la selezione.
        const g = groupRows.get(id);
        if (g) openSoon(() => openGroup(id, g));
      },
      ensureAnchor: gsel.ensureAnchor,
      selectAll: () => void selectAll(),
      invert: () => void invertSelection(),
      clear: selectedCount > 0 ? clearSelection : undefined,
    });
  };

  const identityByUid = useMemo(() => new Map(identityRows.map((r) => [r.uid, r])), [identityRows]);
  const idCursorUid =
    idCursor ??
    (target?.kind === "identity"
      ? target.uid
      : target?.kind === "group"
        ? (identityRows.find((r) => r.person === target.id)?.uid ?? null)
        : null);
  const idCursorIndex = idCursorUid == null ? -1 : identityUids.indexOf(idCursorUid);
  const onIdentitiesKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    const mod = e.ctrlKey || e.metaKey;
    const plain = !mod && !e.shiftKey && !e.altKey;
    if (plain && (e.key === "Enter" || e.key === " ") && idCursorUid != null) {
      const row = identityByUid.get(idCursorUid);
      if (!row) return;
      e.preventDefault();
      if (e.key === " ") isel.click(idCursorUid, { ctrl: false, shift: false });
      selectIdentity(row);
      return;
    }
    handleSelectionKey(e, {
      layout: "list",
      pageSize: listPage(),
      count: identityUids.length,
      cursor: idCursorIndex,
      idAt: (i) => identityUids[i],
      moveTo: (i) => {
        const uid = identityUids[i];
        if (uid == null) return;
        setIdCursor(uid);
        idVirt.scrollToIndex(i, { align: "auto" });
      },
      select: (uid, mods) => {
        isel.click(uid, mods);
        if (mods.ctrl && !mods.shift) return;
        const row = identityByUid.get(uid);
        if (row) openSoon(() => selectIdentity(row));
      },
      ensureAnchor: isel.ensureAnchor,
      selectAll: () => void selectAll(),
      invert: () => void invertSelection(),
      clear: selectedCount > 0 ? clearSelection : undefined,
    });
  };
  // Menu contestuale dell'elenco (modello comune): il clic destro ha già applicato la regola della
  // selezione sulla voce; le azioni agiscono su tutta la selezione e ne dicono il numero.
  const listMenuItems: MenuItem[] =
    section === "review"
      ? [
          {
            label: t("people.bulk.assignMenu", { count: selectedIds.length }),
            disabled: selectedIds.length === 0,
            onSelect: () => setDialog({ kind: "assign", ids: selectedRef.current }),
          },
        ]
      : section === "people"
        ? [
            {
              label: t("people.bulk.mergeMenu", { count: selectedUids.length }),
              disabled: selectedUids.length === 0,
              onSelect: () => setDialog({ kind: "merge", uids: selectedUids }),
            },
          ]
        : [];

  /** Clic su uno spazio vuoto dell'elenco: svuota la selezione (l'elemento aperto resta). */
  const onListEmptyClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest('button, [role="option"]')) return;
    if (e.ctrlKey || e.metaKey || e.shiftKey) return;
    if (selectedCount > 0) clearSelection();
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

  // Ctrl+A, Ctrl+Maiusc+I ed Esc agiscono sull'elenco quando è il riquadro attivo (ultimo clic o
  // focus dentro l'elenco); altrimenti sui volti.
  useActivePane(
    {
      label: t("people.listLabel"),
      selectAll,
      invert: invertSelection,
      clear: clearSelection,
      count: () => selectedCount,
    },
    asideRef,
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
      isel.clear();
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

      <ContextMenu label={t("people.listLabel")} items={listMenuItems}>
        <div
          ref={scrollRef}
          data-testid="group-list-scroll"
          className="people-list-scroll"
          onContextMenu={(e) => {
            // Fuori da una voce, o senza azioni per la sezione: niente menu.
            if (!listMenuItems.length || !(e.target as HTMLElement).closest('[role="option"]'))
              e.preventDefault();
          }}
        >
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
                    role="listbox"
                    aria-multiselectable
                    aria-label={t("identities.title")}
                    tabIndex={0}
                    aria-activedescendant={
                      idCursorIndex >= 0
                        ? identityOptionId(identityUids[idCursorIndex]!)
                        : undefined
                    }
                    onKeyDown={onIdentitiesKey}
                    onClick={onListEmptyClick}
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
                      return (
                        <div
                          key={r.uid}
                          className="people-virt-row"
                          style={{ height: ROW_H, transform: `translateY(${vi.start}px)` }}
                        >
                          <div
                            role="option"
                            id={identityOptionId(r.uid)}
                            aria-selected={checked}
                            aria-current={current || undefined}
                            data-cursor={r.uid === idCursorUid || undefined}
                            onClick={(e) => clickIdentity(r, e)}
                            onContextMenu={() => {
                              // Clic destro: su una persona non selezionata, selezione solo di quella.
                              if (selectForContextMenu(isel.selection, r.uid) !== isel.selection)
                                isel.setIds([r.uid], r.uid);
                              setIdCursor(r.uid);
                            }}
                            className="people-list-item people-list-item--id"
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
                                {[
                                  r.group,
                                  r.person != null && r.photos > 0
                                    ? t("people.photos", { count: r.photos })
                                    : t("people.noPhotosInLibrary"),
                                ]
                                  .filter(Boolean)
                                  .join(" · ")}
                              </span>
                            </span>
                          </div>
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
                    tabIndex={0}
                    aria-activedescendant={
                      groupCursorId != null && groupCursorIndex >= 0
                        ? groupOptionId(groupCursorId)
                        : undefined
                    }
                    onKeyDown={onGroupsKey}
                    onClick={onListEmptyClick}
                    className="people-virt"
                    style={{ height: virt.getTotalSize() }}
                  >
                    {vitems.map((vi) => {
                      const item = virtItems[vi.index];
                      if (!item) return null;
                      const checked = item.kind === "group" && selectedIdSet.has(item.group.id);
                      const current =
                        item.kind === "group" &&
                        target?.kind === "group" &&
                        target.id === item.group.id;
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
                                id={groupOptionId(item.group.id)}
                                data-group-id={item.group.id}
                                aria-selected={checked}
                                aria-current={current || undefined}
                                data-cursor={item.group.id === groupCursorId || undefined}
                                onClick={(e) => selectGroup(item.group.id, e, item.group)}
                                onContextMenu={() => {
                                  // Clic destro: su un gruppo non selezionato, selezione solo di quello.
                                  const id = item.group.id;
                                  if (selectForContextMenu(gsel.selection, id) !== gsel.selection)
                                    gsel.setIds([id], id);
                                  setGroupCursor(id);
                                }}
                                className={
                                  section === "ignored"
                                    ? "people-list-item people-list-item--ignored"
                                    : "people-list-item people-list-item--group"
                                }
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
                                  <span className="muted people-muted-sm">
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
      </ContextMenu>

      {dialog?.kind === "merge" && (
        <PersonPicker {...mergeProps(dialog.uids)} onClose={() => setDialog(null)} />
      )}
      {dialog?.kind === "assign" && (
        <PersonPicker {...assignProps(dialog.ids)} onClose={() => setDialog(null)} />
      )}
    </aside>
  );
});
