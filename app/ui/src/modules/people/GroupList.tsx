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
  type IdentityRow,
  type PeopleSection,
  type PeopleSections,
} from "../../lib/ipc";
import { QueryView } from "../../components/feedback/QueryView";
import { Button } from "../../components/ui";
import { notifyError } from "../../store/toasts";
import { notifyPeopleUndo, refreshPeople } from "./peopleUndo";
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

function Cover({ group }: { group: GroupRow }) {
  const [failed, setFailed] = useState(false);
  const cover = group.cover;
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  return (
    <span
      aria-hidden
      style={{
        display: "block",
        width: 44,
        height: 44,
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
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          style={{ width: "100%", height: "100%", objectFit: "cover" }}
        />
      )}
    </span>
  );
}

function groupTitle(g: GroupRow, t: (k: string, o?: Record<string, unknown>) => string) {
  if (g.name) return g.name;
  return t("people.groupOfFaces", { count: g.faces });
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
    /** Elenco corrente dei gruppi (per tastiera e refresh senza perdere la selezione). */
    onCatalogChange?: (rows: GroupRow[]) => void;
  }
>(function GroupList(
  { section, onSectionChange, target, selectedIds, onSelect, onCreateIdentity, onCatalogChange },
  ref,
) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<GroupSort>("photos_desc");
  const [groupFilter, setGroupFilter] = useState<string | null>(null);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [restoreBusy, setRestoreBusy] = useState<number | null>(null);
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
    queryKey: ["identities", debouncedQuery.trim()],
    queryFn: () => commands.listIdentities(debouncedQuery.trim() || null),
    enabled: isTauri() && section === "people",
  });

  const groups = useInfiniteQuery({
    queryKey: ["people-groups", section, debouncedQuery, sort],
    queryFn: ({ pageParam }) =>
      commands.listGroups(section, debouncedQuery || null, sort, pageParam, PAGE),
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
    // In jsdom / flex annidato l'altezza misurata può essere 0: resta l'initialRect.
    observeElementRect: (_instance, cb) => {
      const el = scrollRef.current;
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
    },
  });

  useImperativeHandle(ref, () => ({
    scrollToGroup(id: number) {
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
      className="btn"
      onClick={() => onSectionChange(id)}
      style={{
        flex: "1 1 auto",
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-start",
        gap: 0,
        padding: "6px 10px",
        minHeight: 0,
        textAlign: "left",
        background: section === id ? "var(--pc-raised)" : "transparent",
        borderColor: section === id ? "var(--pc-border)" : "transparent",
      }}
    >
      <b style={{ fontWeight: 600 }}>{label}</b>
      <span className="mono muted" style={{ fontSize: 12 }}>
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

  const emptyMsg =
    section === "review"
      ? t("people.emptyReview")
      : section === "people"
        ? t("people.emptyPeople")
        : t("people.emptyIgnored");

  const idGroups = identities.data?.groups ?? [];
  const everyone = identities.data?.identities ?? [];

  return (
    <aside
      aria-label={t("people.listLabel")}
      style={{
        flex: "1 1 260px",
        maxWidth: 320,
        minWidth: 0,
        borderRight: "1px solid var(--pc-border)",
        background: "var(--pc-panel-alt)",
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
      }}
    >
      <div
        style={{
          padding: 12,
          display: "flex",
          flexDirection: "column",
          gap: 10,
          borderBottom: "1px solid var(--pc-border)",
        }}
      >
        <div role="tablist" style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
          {tab("review", t("people.sectionReview"), counts.review)}
          {tab("people", t("people.sectionPeople"), peopleCount)}
          {tab("ignored", t("people.sectionIgnored"), counts.ignored)}
        </div>
        {section === "people" ? (
          <>
            <button
              className="btn btn-primary"
              onClick={() => onCreateIdentity?.()}
              style={{ alignSelf: "stretch" }}
            >
              {t("identities.newButton")}
            </button>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
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
              className="field"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("identities.search")}
              aria-label={t("identities.search")}
              style={{ width: "100%" }}
            />
          </>
        ) : (
          <>
            <input
              className="field"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("people.filterPlaceholder")}
              aria-label={t("people.filterPlaceholder")}
              style={{ width: "100%" }}
            />
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {SORTS.map((s) => (
                <button
                  key={s}
                  type="button"
                  className="chip"
                  aria-pressed={sort === s}
                  onClick={() => setSort(s)}
                  style={{
                    cursor: "pointer",
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
      </div>

      <div
        ref={scrollRef}
        data-testid="group-list-scroll"
        style={{ flex: 1, overflow: "auto", minHeight: 0, padding: 6 }}
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
                  role="list"
                  aria-label={t("identities.title")}
                  style={{ display: "flex", flexDirection: "column", gap: 2 }}
                >
                  {identityRows.map((r) => {
                    const selected =
                      (target?.kind === "identity" && target.uid === r.uid) ||
                      (target?.kind === "group" && r.person != null && target.id === r.person);
                    return (
                      <div key={r.uid} role="listitem">
                        <button
                          type="button"
                          aria-current={selected || undefined}
                          onClick={() => selectIdentity(r)}
                          style={{
                            display: "grid",
                            gridTemplateColumns: "44px 1fr auto",
                            gap: 10,
                            alignItems: "center",
                            width: "100%",
                            padding: 6,
                            borderRadius: 6,
                            border: selected
                              ? "1px solid var(--pc-selection-border)"
                              : "1px solid transparent",
                            background: selected ? "var(--pc-selection-bg)" : "transparent",
                            textAlign: "left",
                            cursor: "pointer",
                            color: "inherit",
                            font: "inherit",
                          }}
                        >
                          <span
                            aria-hidden
                            style={{
                              width: 44,
                              height: 44,
                              borderRadius: "50%",
                              background: "var(--pc-thumb-bg)",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              fontWeight: 700,
                              fontSize: 14,
                              color: "var(--pc-text-soft)",
                            }}
                          >
                            {r.name
                              .split(/\s+/)
                              .filter(Boolean)
                              .slice(0, 2)
                              .map((w) => w[0]?.toUpperCase())
                              .join("")}
                          </span>
                          <span style={{ minWidth: 0 }}>
                            <span
                              style={{
                                display: "block",
                                fontWeight: 600,
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                              }}
                            >
                              {r.name}
                            </span>
                            <span className="muted" style={{ fontSize: 12 }}>
                              {r.group ?? t("identities.notLinkedShort")}
                              {r.faces
                                ? ` · ${t("identities.refCount", { count: r.faces })}`
                                : ` · ${t("identities.noRefs")}`}
                            </span>
                          </span>
                          <span className="mono muted" style={{ fontSize: 12 }}>
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
                  style={{ height: virt.getTotalSize(), position: "relative" }}
                >
                  {vitems.map((vi) => {
                    const item = virtItems[vi.index];
                    if (!item) return null;
                    return (
                      <div
                        key={vi.key}
                        style={{
                          position: "absolute",
                          top: 0,
                          left: 0,
                          right: 0,
                          height: vi.size,
                          transform: `translateY(${vi.start}px)`,
                        }}
                      >
                        {item.kind === "label" && (
                          <div
                            role="presentation"
                            className="label"
                            style={{
                              padding: "10px 8px 4px",
                              fontSize: 12,
                              letterSpacing: "0.06em",
                            }}
                          >
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
                                  : selectedIds.includes(item.group.id)
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
                              style={{
                                display: "grid",
                                gridTemplateColumns:
                                  section === "ignored" ? "44px 1fr" : "44px 1fr auto",
                                gap: 10,
                                alignItems: "center",
                                minWidth: 0,
                                padding: 6,
                                borderRadius: 6,
                                border:
                                  (target?.kind === "group" && target.id === item.group.id) ||
                                  selectedIds.includes(item.group.id)
                                    ? "1px solid var(--pc-selection-border)"
                                    : "1px solid transparent",
                                background:
                                  (target?.kind === "group" && target.id === item.group.id) ||
                                  selectedIds.includes(item.group.id)
                                    ? "var(--pc-selection-bg)"
                                    : "transparent",
                                cursor: "pointer",
                                color: "inherit",
                                font: "inherit",
                                textAlign: "left",
                              }}
                            >
                              <Cover group={item.group} />
                              <span style={{ minWidth: 0 }}>
                                <span
                                  style={{
                                    display: "block",
                                    fontWeight: 600,
                                    overflow: "hidden",
                                    textOverflow: "ellipsis",
                                    whiteSpace: "nowrap",
                                  }}
                                >
                                  {groupTitle(item.group, t)}
                                </span>
                                <span className="muted" style={{ fontSize: 12 }}>
                                  {groupSub(item.group, t)}
                                </span>
                              </span>
                              {section !== "ignored" && (
                                <span className="mono muted" style={{ fontSize: 12 }}>
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
                            style={{
                              display: "grid",
                              gridTemplateColumns: "44px 1fr",
                              gap: 10,
                              alignItems: "center",
                              width: "100%",
                              padding: 6,
                              borderRadius: 6,
                              border:
                                target?.kind === "singles"
                                  ? "1px solid var(--pc-selection-border)"
                                  : "1px solid transparent",
                              background:
                                target?.kind === "singles"
                                  ? "var(--pc-selection-bg)"
                                  : "transparent",
                              textAlign: "left",
                              cursor: "pointer",
                              color: "inherit",
                              font: "inherit",
                            }}
                          >
                            <span
                              aria-hidden
                              style={{
                                width: 44,
                                height: 44,
                                borderRadius: "50%",
                                background: "var(--pc-thumb-bg)",
                              }}
                            />
                            <span style={{ minWidth: 0 }}>
                              <span style={{ display: "block", fontWeight: 600 }}>
                                {t("people.singlesLabel", { count: item.count })}
                              </span>
                              <span className="muted" style={{ fontSize: 12 }}>
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
                            style={{
                              display: "grid",
                              gridTemplateColumns: "44px 1fr",
                              gap: 10,
                              alignItems: "center",
                              width: "100%",
                              padding: 6,
                              borderRadius: 6,
                              border:
                                target?.kind === "discarded"
                                  ? "1px solid var(--pc-selection-border)"
                                  : "1px solid transparent",
                              background:
                                target?.kind === "discarded"
                                  ? "var(--pc-selection-bg)"
                                  : "transparent",
                              textAlign: "left",
                              cursor: "pointer",
                              color: "inherit",
                              font: "inherit",
                            }}
                          >
                            <span
                              aria-hidden
                              style={{
                                width: 44,
                                height: 44,
                                borderRadius: "50%",
                                background: "var(--pc-thumb-bg)",
                              }}
                            />
                            <span style={{ minWidth: 0 }}>
                              <span style={{ display: "block", fontWeight: 600 }}>
                                {t("people.discardedAllLabel", { count: item.count })}
                              </span>
                              <span className="muted" style={{ fontSize: 12 }}>
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
    </aside>
  );
});
