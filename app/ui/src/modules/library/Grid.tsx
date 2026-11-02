import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { useQuery } from "@tanstack/react-query";
import { commands, isTauri, type GridItem, type Target } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { latestOnly, retryUnlessSuperseded, useLibraryCount } from "../../lib/latest";
import {
  clickSelect,
  emptyFilter,
  idsSelection,
  rectangleSelect,
  selCount,
  selHas,
  selIds,
  selSet,
  selTarget,
  thumbPx,
  useLibrary,
  type Selection,
} from "../../store/library";
import { useGridPages, windowAction } from "./gridPages";
import { byBase, displayTotal, locate, toBase, type Expansion } from "./similarView";
import { useCatalogAction } from "./useCatalogAction";
import { thumbUrl } from "./thumbUrl";
import { GridContextMenu } from "./GridContextMenu";
import { useOpenWith } from "./useOpenWith";
import { usePlaceAction } from "./usePlaceAction";
import { colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";

/** Chip dei tag in basso sulla miniatura (al massimo 3, poi "+N"). */
export function ThumbTags({ tags, color }: { tags: string[]; color: (name: string) => string }) {
  if (!tags.length) return null;
  const shown = tags.slice(0, 3);
  return (
    <span
      style={{
        position: "absolute",
        left: 6,
        right: 34,
        bottom: 6,
        display: "flex",
        flexWrap: "wrap",
        gap: 4,
        overflow: "hidden",
        maxHeight: 40,
      }}
    >
      {shown.map((t) => (
        <TagChip key={t} name={t} color={color(t)} compact />
      ))}
      {tags.length > shown.length && (
        <TagChip name={`+${tags.length - shown.length}`} color="gray" compact />
      )}
    </span>
  );
}

export { PAGE_SIZE } from "./gridPages";
const GAP = 12;
const PAD = 16;
const CAPTION = 26;

function Thumb({
  item,
  size,
  selected,
  onClick,
  onDoubleClick,
  onMenu,
  tagColor,
  group,
  cover,
}: {
  item: GridItem;
  size: number;
  selected: boolean;
  /** Elemento che rappresenta un gruppo di foto simili (griglia raggruppata). */
  group?: { count: number; expanded: boolean; onToggle: () => void };
  /** Foto principale di un gruppo aperto. */
  cover?: boolean;
  onClick: (e: MouseEvent) => void;
  onDoubleClick?: () => void;
  /** Menu contestuale alle coordinate indicate (clic destro, tasto Menu, Maiusc+F10). */
  onMenu?: (x: number, y: number) => void;
  tagColor: (name: string) => string;
}) {
  const { t } = useTranslation();
  const url = thumbUrl(item.thumb);
  const [failed, setFailed] = useState(false);
  return (
    <button
      type="button"
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onContextMenu={(e) => {
        e.preventDefault();
        onMenu?.(e.clientX, e.clientY);
      }}
      onKeyDown={(e) => {
        if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
          e.preventDefault();
          const r = e.currentTarget.getBoundingClientRect();
          onMenu?.(r.left + 16, r.top + 16);
        }
      }}
      aria-pressed={selected}
      data-testid={`thumb-${item.id}`}
      title={
        item.excludedAt
          ? `${item.name} · ${t("exclusions.date", { date: new Date(item.excludedAt).toLocaleString() })}`
          : item.name
      }
      style={{
        width: size,
        padding: 0,
        margin: 0,
        borderRadius: 6,
        overflow: "hidden",
        cursor: "pointer",
        textAlign: "left",
        background: "var(--pc-panel)",
        border: `2px solid ${selected ? "var(--pc-accent)" : cover ? "var(--pc-warn)" : "var(--pc-raised)"}`,
        color: "inherit",
      }}
    >
      <div
        style={{
          position: "relative",
          height: Math.round(size * 0.75),
          background: "var(--pc-thumb-bg)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--pc-text-placeholder)",
          fontSize: 12,
        }}
      >
        {url && !failed ? (
          <img
            src={url}
            alt=""
            loading="lazy"
            decoding="async"
            onError={() => setFailed(true)}
            style={{ width: "100%", height: "100%", objectFit: "cover" }}
          />
        ) : (
          <span>{item.name.split(".").pop()?.toUpperCase()}</span>
        )}
        <span style={{ position: "absolute", top: 6, left: 6, display: "flex", gap: 4 }}>
          {item.faces > 0 && (
            <span
              style={{
                padding: "1px 6px",
                borderRadius: 4,
                background: "rgba(20,20,22,0.85)",
                color: "#E8E8EA",
                fontSize: 11,
              }}
            >
              {item.faces}
            </span>
          )}
        </span>
        <span
          style={{
            position: "absolute",
            top: 6,
            right: 6,
            padding: "1px 6px",
            borderRadius: 4,
            background: "rgba(20,20,22,0.85)",
            fontSize: 11,
            color: item.status === "missing" ? "#F2A38F" : item.analyzed ? "#4FC3B0" : "#E0B252",
          }}
        >
          {item.status === "missing"
            ? t("library.missingBadge")
            : item.analyzed
              ? t("library.aiBadge")
              : t("library.queuedBadge")}
        </span>
        <ThumbTags tags={item.tags} color={tagColor} />
        {group && (
          <span
            role="button"
            tabIndex={0}
            aria-expanded={group.expanded}
            aria-label={t(group.expanded ? "similar.collapse" : "similar.expand", {
              count: group.count,
            })}
            data-testid={`group-badge-${item.id}`}
            onClick={(e) => {
              e.stopPropagation();
              group.onToggle();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                e.stopPropagation();
                group.onToggle();
              }
            }}
            style={{
              position: "absolute",
              left: 6,
              bottom: 6,
              padding: "1px 7px",
              borderRadius: 10,
              background: "var(--pc-accent)",
              color: "var(--pc-on-accent, #fff)",
              fontSize: 12,
              fontWeight: 600,
            }}
          >
            {group.expanded ? "−" : ""}
            {group.count}
          </span>
        )}
        {cover && (
          <span
            data-testid={`cover-${item.id}`}
            style={{
              position: "absolute",
              left: 6,
              top: 30,
              padding: "1px 6px",
              borderRadius: 4,
              background: "rgba(20,20,22,0.85)",
              color: "var(--pc-warn)",
              fontSize: 11,
            }}
          >
            {t("similar.cover")}
          </span>
        )}
        {item.excludedAt != null && (
          <span
            style={{
              position: "absolute",
              top: 4,
              left: 4,
              background: "var(--pc-panel)",
              padding: 3,
              fontSize: 11,
            }}
          >
            {t("exclusions.date", { date: new Date(item.excludedAt).toLocaleDateString() })}
          </span>
        )}
        {selected && (
          <span
            aria-hidden="true"
            style={{
              position: "absolute",
              bottom: 6,
              right: 6,
              width: 20,
              height: 20,
              borderRadius: "50%",
              background: "var(--pc-accent)",
            }}
          />
        )}
      </div>
      <div
        className="mono"
        title={item.name}
        style={{
          height: CAPTION,
          padding: "6px 8px",
          fontSize: 11,
          color: "var(--pc-text-soft)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {item.name}
      </div>
    </button>
  );
}

/** Segnaposto di un elemento non ancora caricato (fuori dalla finestra di pagine). */
function Placeholder({ size }: { size: number }) {
  return (
    <div
      aria-hidden="true"
      style={{
        width: size,
        height: Math.round(size * 0.75) + CAPTION + 4,
        borderRadius: 6,
        background: "var(--pc-panel)",
        border: "2px solid var(--pc-raised)",
      }}
    />
  );
}

/**
 * Griglia virtualizzata a memoria limitata: l'altezza viene dal conteggio dei risultati, in
 * memoria restano al massimo MAX_PAGES pagine attorno alla vista (gli altri elementi sono
 * segnaposto finché non si arriva lì).
 */
export function LibraryGrid({ initialWidth }: { initialWidth?: number }) {
  const { t } = useTranslation();
  const {
    filter,
    sort,
    thumbSize,
    selection,
    focused,
    setSelection,
    replaceSelection,
    scrollTop,
    setScrollTop,
  } = useLibrary();
  const catalogAction = useCatalogAction();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const size = thumbPx(thumbSize);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(initialWidth ?? 800);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const drag = useRef<{ x: number; y: number; initial: Selection } | null>(null);
  const tagInfo = useTagOverview().data;
  const tagColor = (name: string) => colorOf(tagInfo, name);
  const [menu, setMenu] = useState<{ x: number; y: number; ids: number[]; target: Target } | null>(
    null,
  );
  const openWith = useOpenWith();
  const place = usePlaceAction();
  const closeMenu = useCallback(() => setMenu(null), []);
  const actionError = openWith.error || place.error;

  // Punto di partenza della finestra di pagine: cambia solo con un salto lontano.
  // Griglia raggruppata: un elemento per gruppo di foto simili (i conteggi restano sulle foto).
  const groupSimilar = useLibrary((s) => s.groupSimilar);
  const viewFilter = useMemo(() => ({ ...filter, groupSimilar }), [filter, groupSimilar]);
  const viewKey = JSON.stringify([viewFilter, sort]);
  const [anchor, setAnchor] = useState({ key: viewKey, at: 0 });
  const at = anchor.key === viewKey ? anchor.at : 0;
  const { pages, start, items } = useGridPages(viewFilter, sort, at);
  const count = useLibraryCount(filter);
  const gridCount = useQuery({
    queryKey: ["library-grid-count", viewFilter],
    queryFn: () =>
      latestOnly("library-grid-count", (tag) => commands.libraryGridCount(viewFilter, tag)),
    enabled: isTauri() && groupSimilar,
    retry: retryUnlessSuperseded,
  });
  const baseTotal = Math.max(
    (groupSimilar ? gridCount.data : count.data) ?? 0,
    start + items.length,
  );
  // Gruppi aperti: le loro foto compaiono dopo l'elemento che li rappresenta.
  const [opened, setOpened] = useState<{ key: string; list: Expansion[] }>({
    key: viewKey,
    list: [],
  });
  const expansions = useMemo(
    () => (opened.key === viewKey && groupSimilar ? byBase(opened.list) : []),
    [opened, viewKey, groupSimilar],
  );
  const total = displayTotal(baseTotal, expansions);
  const at_ = (d: number): { item?: GridItem; member?: boolean } => {
    const s = locate(d, expansions);
    if ("member" in s) return { item: s.member, member: true };
    return { item: s.base >= start ? items[s.base - start] : undefined };
  };
  const order = useMemo(() => items.map((i) => i.id), [items]);
  const toggleGroup = (it: GridItem, base: number) => {
    const group = it.similarGroup;
    if (group == null) return;
    const list = opened.key === viewKey ? opened.list : [];
    if (list.some((e) => e.group === group)) {
      setOpened({ key: viewKey, list: list.filter((e) => e.group !== group) });
      return;
    }
    void commands.similarGroupMembers(group, filter, sort).then((page) => {
      // La principale resta al suo posto: dopo di lei le altre foto del gruppo.
      const members = page.items.filter((m) => m.id !== it.id);
      setOpened((o) => ({
        key: viewKey,
        list: [...(o.key === viewKey ? o.list : []), { group, base, members }],
      }));
    });
  };
  // Selezionare un gruppo chiuso seleziona tutte le sue foto presenti nei risultati.
  const selectGroup = (it: GridItem) => {
    if (it.similarGroup == null) return;
    void commands
      .similarGroupMembers(it.similarGroup, filter, sort)
      .then((page) => setSelection(page.items.map((m) => m.id), it.id));
  };

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([e]) => setWidth(e.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const cols = Math.max(1, Math.floor((width - PAD * 2 + GAP) / (size + GAP)));
  // Le colonne si allargano per riempire la riga: nessuno spazio vuoto a destra.
  const cell = Math.max(size, Math.floor((width - PAD * 2 - (cols - 1) * GAP) / cols));
  const rows = Math.ceil(total / cols);
  const rowHeight = Math.round(cell * 0.75) + CAPTION + 4 + GAP;
  const virt = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 4,
    initialRect: { width, height: 800 },
    initialOffset: scrollTop,
  });
  useEffect(() => virt.measure(), [rowHeight, cols, virt]);

  const vitems = virt.getVirtualItems();
  const firstRow = vitems[0]?.index ?? 0;
  const lastRow = vitems[vitems.length - 1]?.index ?? 0;
  useEffect(() => {
    if (pages.isFetching || !pages.isSuccess) return;
    const action = windowAction(
      {
        from: toBase(firstRow * cols, expansions),
        to: toBase(Math.min((lastRow + 1) * cols, total) - 1, expansions) + 1,
      },
      { start, len: items.length, hasNext: pages.hasNextPage, hasPrev: pages.hasPreviousPage },
      cols * 3,
    );
    if (action === "next") void pages.fetchNextPage();
    else if (action === "prev") void pages.fetchPreviousPage();
    else if (action) setAnchor({ key: viewKey, at: action.seek });
  }, [firstRow, lastRow, cols, start, items.length, pages, viewKey, expansions, total]);

  const explicit = selIds(selection);
  const onClick = (id: number) => (e: MouseEvent) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (selection.mode === "filter" && ctrl) {
      // "Tutti tranne…": Ctrl+clic aggiunge o toglie un'eccezione.
      replaceSelection(selSet(selection, [id], !selHas(selection, id)), id);
      return;
    }
    const next = clickSelect(order, explicit ?? [], focused, id, { ctrl, shift: e.shiftKey });
    setSelection(next, id);
  };
  // Clic destro: sulla selezione agisce su tutta, altrimenti seleziona solo la miniatura.
  const onMenu = (id: number) => (x: number, y: number) => {
    if (selHas(selection, id)) {
      // "Apri con" lavora sugli elementi caricati; le azioni bulk sull'intera selezione.
      const ids = explicit ?? order.filter((i) => selHas(selection, i));
      setMenu({ x, y, ids, target: selTarget(selection) });
    } else {
      setSelection([id], id);
      setMenu({ x, y, ids: [id], target: { kind: "ids", ids: [id] } });
    }
  };

  if (pages.isSuccess && items.length === 0 && start === 0 && (count.data ?? 0) === 0) {
    const filtered = JSON.stringify(filter) !== JSON.stringify(emptyFilter());
    return (
      <div className="muted" style={{ padding: 32 }}>
        {filtered ? t("library.noResults") : t("library.empty")}
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      tabIndex={0}
      onKeyDown={(e) => {
        if ((e.target as HTMLElement).closest("input,textarea,select,[contenteditable=true]"))
          return;
        const restore = filter.excludedFrom != null;
        if (
          (e.key === "Delete" && filter.catalogId != null && !restore) ||
          (e.key === "Enter" && restore)
        ) {
          e.preventDefault();
          const id = filter.excludedFrom ?? filter.catalogId!;
          void catalogAction.act(
            { target: selTarget(selection), count: selCount(selection) },
            id,
            catalogs.data?.find((c) => c.catalog.id === id)?.catalog.name ?? "",
            restore,
          );
        }
      }}
      data-testid="grid-scroll"
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
      style={{ flex: 1, overflow: "auto", position: "relative" }}
      aria-label={t("library.count", { n: fmtInt(total) })}
      role="region"
      onPointerDown={(e) => {
        if (e.button !== 0 || ((e.target as HTMLElement).closest("button") && !e.altKey)) return;
        const r = e.currentTarget.getBoundingClientRect();
        drag.current = {
          x: e.clientX - r.left,
          y: e.clientY - r.top + e.currentTarget.scrollTop,
          initial: e.ctrlKey || e.metaKey ? selection : idsSelection(),
        };
        e.currentTarget.setPointerCapture?.(e.pointerId);
        e.preventDefault();
      }}
      onPointerMove={(e) => {
        const d = drag.current;
        if (!d) return;
        const el = e.currentTarget,
          r = el.getBoundingClientRect();
        if (e.clientY > r.bottom - 30) el.scrollTop += 16;
        else if (e.clientY < r.top + 30) el.scrollTop -= 16;
        const x = e.clientX - r.left,
          y = e.clientY - r.top + el.scrollTop;
        const b = {
          x: Math.min(x, d.x),
          y: Math.min(y, d.y),
          w: Math.abs(x - d.x),
          h: Math.abs(y - d.y),
        };
        setBox(b);
        const inside = rectangleSelect(
          order,
          b,
          { cols, cell, rowHeight, gap: GAP, pad: PAD },
          start,
        );
        replaceSelection(selSet(d.initial, inside, true));
      }}
      onPointerUp={() => {
        drag.current = null;
        setBox(null);
      }}
      onPointerCancel={() => {
        drag.current = null;
        setBox(null);
      }}
    >
      <div style={{ height: virt.getTotalSize() + PAD * 2, position: "relative" }}>
        {box && (
          <div
            aria-hidden="true"
            style={{
              position: "absolute",
              left: box.x,
              top: box.y,
              width: box.w,
              height: box.h,
              border: "1px solid var(--pc-accent)",
              background: "var(--pc-accent-soft-bg)",
              opacity: 0.5,
              pointerEvents: "none",
              zIndex: 2,
            }}
          />
        )}
        {vitems.map((row) => {
          const first = row.index * cols;
          const cells = [];
          for (let i = first; i < Math.min(first + cols, total); i++) {
            const { item: it, member } = at_(i);
            const base = toBase(i, expansions);
            const grouped =
              groupSimilar && !member && it?.similarGroup != null && (it.similarCount ?? 1) > 1;
            const open = grouped && expansions.some((e) => e.group === it?.similarGroup);
            cells.push(
              it ? (
                <Thumb
                  key={it.id}
                  item={it}
                  size={cell}
                  selected={selHas(selection, it.id)}
                  group={
                    grouped
                      ? {
                          count: it.similarCount ?? 1,
                          expanded: open,
                          onToggle: () => toggleGroup(it, base),
                        }
                      : undefined
                  }
                  cover={open && it.similarCover}
                  onClick={
                    grouped && !open
                      ? (e) => (e.ctrlKey || e.metaKey || e.shiftKey ? onClick(it.id)(e) : selectGroup(it))
                      : onClick(it.id)
                  }
                  onDoubleClick={() => void openWith.open([it.id], { kind: "double_click" })}
                  onMenu={onMenu(it.id)}
                  tagColor={tagColor}
                />
              ) : (
                <Placeholder key={`p${i}`} size={cell} />
              ),
            );
          }
          return (
            <div
              key={row.key}
              style={{
                position: "absolute",
                top: row.start + PAD,
                left: PAD,
                right: PAD,
                display: "flex",
                gap: GAP,
              }}
            >
              {cells}
            </div>
          );
        })}
      </div>
      {(pages.isFetchingNextPage || pages.isFetchingPreviousPage) && (
        <div className="muted" style={{ position: "sticky", bottom: 8, textAlign: "center" }}>
          <Icon.Spinner size={14} />
        </div>
      )}
      {actionError && (
        <div
          role="alert"
          style={{
            position: "sticky",
            bottom: 8,
            margin: "0 16px",
            padding: "6px 10px",
            borderRadius: 4,
            background: "var(--pc-raised)",
            border: "1px solid var(--pc-border)",
          }}
          onClick={openWith.clearError}
        >
          {actionError}
        </div>
      )}
      {menu && (
        <GridContextMenu
          x={menu.x}
          y={menu.y}
          ids={menu.ids}
          target={menu.target}
          onOpen={(ids, target) => void openWith.open(ids, target)}
          onPlace={(target, p) => void place.apply(target, p)}
          onClose={closeMenu}
        />
      )}
    </div>
  );
}
