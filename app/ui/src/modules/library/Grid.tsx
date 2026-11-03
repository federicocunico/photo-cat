import { QueryView } from "../../components/feedback/QueryView";
import { Skeleton } from "../../components/feedback/Skeleton";
import { RefreshBar } from "../../components/feedback/RefreshBar";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { useQuery } from "@tanstack/react-query";
import { commands, isTauri, type GridItem, type Target } from "../../lib/ipc";
import { fmtDate, fmtDateTime, fmtInt } from "../../lib/format";
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
import { Viewer } from "../../components/viewer/Viewer";
import { useBulkJob } from "./bulkJob";
import { activeSet } from "../tags/data";
import { hotkeyAction } from "../tags/useTagHotkeys";
import { colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";

/** Chip dei tag in basso sulla miniatura (al massimo 3, poi "+N"). */
export function ThumbTags({ tags, color }: { tags: string[]; color: (name: string) => string }) {
  if (!tags.length) return null;
  const shown = tags.slice(0, 3);
  return (
    <span className="thumb-tags">
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
  cursor,
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
  /** Elemento con il focus della tastiera (distinto dalla selezione). */
  cursor: boolean;
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
    <div className="thumb-cell" style={{ width: size }}>
      <div
        role="option"
        id={thumbId(item.id)}
        aria-selected={selected}
        aria-label={item.name}
        data-cursor={cursor || undefined}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
        onContextMenu={(e) => {
          e.preventDefault();
          onMenu?.(e.clientX, e.clientY);
        }}
        data-testid={`thumb-${item.id}`}
        title={
          item.excludedAt
            ? `${item.name} · ${t("exclusions.date", { date: fmtDateTime(item.excludedAt) })}`
            : item.name
        }
        className="thumb-frame"
        style={{
          width: size,
          border: `2px solid ${selected ? "var(--pc-accent)" : cover ? "var(--pc-warn)" : "var(--pc-raised)"}`,
        }}
      >
        <div className="thumb-media" style={{ height: Math.round(size * 0.75) }}>
          {url && !failed ? (
            <img src={url} alt="" loading="lazy" decoding="async" onError={() => setFailed(true)} />
          ) : (
            <span>{item.name.split(".").pop()?.toUpperCase()}</span>
          )}
          <span className="thumb-face">
            {item.faces > 0 && <span className="thumb-badge thumb-badge-face">{item.faces}</span>}
          </span>
          <span
            className="thumb-badge thumb-badge-status"
            style={{
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
          {cover && (
            <span data-testid={`cover-${item.id}`} className="thumb-badge thumb-badge-cover">
              {t("similar.cover")}
            </span>
          )}
          {item.excludedAt != null && (
            <span className="thumb-excluded">
              {t("exclusions.date", { date: fmtDate(item.excludedAt) })}
            </span>
          )}
          {selected && <span aria-hidden="true" className="thumb-check" />}
        </div>
        <div className="mono thumb-caption" title={item.name}>
          {item.name}
        </div>
      </div>
      {group && (
        <button
          type="button"
          className="thumb-group"
          aria-expanded={group.expanded}
          aria-label={t(group.expanded ? "similar.collapse" : "similar.expand", {
            count: group.count,
          })}
          data-testid={`group-badge-${item.id}`}
          style={{ top: Math.round(size * 0.75) - 30 }}
          onClick={group.onToggle}
        >
          {group.expanded && <Icon.Minus size={12} aria-hidden="true" />}
          {group.count}
        </button>
      )}
    </div>
  );
}

const thumbId = (id: number) => `thumb-opt-${id}`;

/** Segnaposto di un elemento non ancora caricato (fuori dalla finestra di pagine). */
function Placeholder({ size }: { size: number }) {
  return (
    <div
      aria-hidden="true"
      className="thumb-placeholder"
      style={{
        width: size,
        height: Math.round(size * 0.75) + CAPTION + 4,
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
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(initialWidth ?? 800);
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const drag = useRef<{ x: number; y: number; initial: Selection } | null>(null);
  const tagInfo = useTagOverview().data;
  const tagColor = (name: string) => colorOf(tagInfo, name);
  const [menu, setMenu] = useState<{ x: number; y: number; ids: number[]; target: Target } | null>(
    null,
  );
  const openWith = useOpenWith();
  const place = usePlaceAction();
  const config = useQuery({
    queryKey: ["config"],
    queryFn: () => commands.getConfig(),
    enabled: isTauri(),
  });
  // Visualizzatore interno: posizione nella vista, null se chiuso.
  const [viewerAt, setViewerAt] = useState<number | null>(null);
  const bulk = useBulkJob();
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
    void commands.similarGroupMembers(it.similarGroup, filter, sort).then((page) =>
      setSelection(
        page.items.map((m) => m.id),
        it.id,
      ),
    );
  };

  useLayoutEffect(() => {
    const el = scrollEl;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([e]) => setWidth(e.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, [scrollEl]);

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
  // Focus della tastiera: indice nella vista, distinto dalla selezione (design D5).
  const [cursorState, setCursorState] = useState({ key: viewKey, at: 0 });
  const cursor = cursorState.key === viewKey ? Math.min(cursorState.at, Math.max(0, total - 1)) : 0;
  const setCursor = (at: number) => setCursorState({ key: viewKey, at });
  // Ancora degli intervalli con Maiusc+freccia.
  const rangeAnchor = useRef<number | null>(null);
  const cursorItem = total > 0 ? at_(cursor).item : undefined;
  const moveCursor = (to: number, extend: boolean) => {
    const n = Math.max(0, Math.min(total - 1, to));
    setCursor(n);
    virt.scrollToIndex(Math.floor(n / cols), { align: "auto" });
    const it = at_(n).item;
    if (!it) return;
    if (extend) {
      const anchor = rangeAnchor.current ?? cursorItem?.id ?? it.id;
      rangeAnchor.current = anchor;
      setSelection(clickSelect(order, [], anchor, it.id, { ctrl: false, shift: true }), it.id);
    } else {
      rangeAnchor.current = it.id;
      useLibrary.setState({ focused: it.id });
    }
  };
  const onGridKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    const page = Math.max(1, Math.floor(e.currentTarget.clientHeight / rowHeight)) * cols;
    const mod = e.ctrlKey || e.metaKey;
    const moves: Record<string, number> = {
      ArrowLeft: cursor - 1,
      ArrowRight: cursor + 1,
      ArrowUp: cursor - cols,
      ArrowDown: cursor + cols,
      PageUp: cursor - page,
      PageDown: cursor + page,
      Home: 0,
      End: total - 1,
    };
    if (e.key in moves && !e.altKey && !mod) {
      e.preventDefault();
      moveCursor(moves[e.key], e.shiftKey);
      return true;
    }
    if (mod && e.key === " " && cursorItem) {
      // Ctrl+Spazio: aggiunge o toglie dalla selezione l'elemento con il focus.
      e.preventDefault();
      const id = cursorItem.id;
      if (selection.mode === "filter")
        replaceSelection(selSet(selection, [id], !selHas(selection, id)), id);
      else
        setSelection(
          clickSelect(order, explicit ?? [], focused, id, { ctrl: true, shift: false }),
          id,
        );
      return true;
    }
    if (mod && (e.key === "a" || e.key === "A") && !e.shiftKey) {
      e.preventDefault();
      void commands.librarySelection(filter).then((base) => useLibrary.getState().selectAll(base));
      return true;
    }
    const restoreView = filter.excludedFrom != null;
    if (
      !mod &&
      !e.altKey &&
      !e.shiftKey &&
      cursorItem &&
      (e.key === " " || (e.key === "Enter" && !restoreView))
    ) {
      // Nelle escluse Invio resta "Reinserisci": il visualizzatore si apre con Spazio.
      e.preventDefault();
      setViewerAt(cursor);
      return true;
    }
    if ((e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) && cursorItem) {
      e.preventDefault();
      const el = document.getElementById(thumbId(cursorItem.id));
      const r = el?.getBoundingClientRect();
      onMenu(cursorItem.id)((r?.left ?? 0) + 16, (r?.top ?? 0) + 16);
      return true;
    }
    return false;
  };
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

  if (isTauri() && (pages.isPending || (pages.isError && !pages.data))) {
    return (
      <QueryView query={pages} skeleton={<Skeleton kind="thumbnail" count={24} />}>
        {() => null}
      </QueryView>
    );
  }

  if (pages.isSuccess && items.length === 0 && start === 0 && (count.data ?? 0) === 0) {
    const filtered = JSON.stringify(filter) !== JSON.stringify(emptyFilter());
    return (
      <div className="muted grid-empty">
        {filtered ? t("library.noResults") : t("library.empty")}
      </div>
    );
  }

  return (
    <div
      aria-busy={pages.isFetching || undefined}
      data-refreshing={pages.isPlaceholderData || undefined}
      ref={(el) => {
        scrollRef.current = el;
        setScrollEl(el);
      }}
      tabIndex={0}
      aria-activedescendant={cursorItem ? thumbId(cursorItem.id) : undefined}
      aria-multiselectable="true"
      onKeyDown={(e) => {
        if ((e.target as HTMLElement).closest("input,textarea,select,[contenteditable=true]"))
          return;
        if (onGridKey(e)) return;
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
      aria-label={
        count.isPending
          ? t("feedback.calculating")
          : t("library.count", { n: fmtInt(count.data ?? total) })
      }
      role="listbox"
      className="grid-scroll"
      onPointerDown={(e) => {
        if (
          e.button !== 0 ||
          ((e.target as HTMLElement).closest('button, [role="option"]') && !e.altKey)
        )
          return;
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
      <RefreshBar active={pages.isFetching && !pages.isPending} />
      {pages.isError && <QueryView query={pages}>{() => null}</QueryView>}
      <div className="grid-virt" style={{ height: virt.getTotalSize() + PAD * 2 }}>
        {box && (
          <div
            aria-hidden="true"
            className="grid-marquee"
            style={{ left: box.x, top: box.y, width: box.w, height: box.h }}
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
                  cursor={i === cursor}
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
                  onClick={(e) => {
                    setCursor(i);
                    rangeAnchor.current = it.id;
                    if (grouped && !open && !(e.ctrlKey || e.metaKey || e.shiftKey))
                      selectGroup(it);
                    else onClick(it.id)(e);
                  }}
                  onDoubleClick={() => {
                    if (config.data?.ui?.double_click === "viewer") {
                      setCursor(i);
                      setViewerAt(i);
                    } else void openWith.open([it.id], { kind: "double_click" });
                  }}
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
              className="grid-row"
              style={{
                top: row.start + PAD,
                left: PAD,
                right: PAD,
                gap: GAP,
              }}
            >
              {cells}
            </div>
          );
        })}
      </div>
      {(pages.isFetchingNextPage || pages.isFetchingPreviousPage) && (
        <div className="muted grid-loading">
          <Icon.Spinner size={14} />
        </div>
      )}
      {actionError && (
        <div role="alert" className="grid-action-error" onClick={openWith.clearError}>
          {actionError}
        </div>
      )}
      {viewerAt != null && (
        <Viewer
          index={viewerAt}
          total={total}
          photo={(() => {
            const it = at_(viewerAt).item;
            if (!it) return null;
            return {
              id: it.id,
              name: it.name,
              src: thumbUrl(it.thumb),
              details: (
                <span>
                  {[it.capturedAt ? fmtDateTime(it.capturedAt.replace(" ", "T")) : null, ...it.tags]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              ),
            };
          })()}
          onIndex={(n) => {
            setViewerAt(n);
            // La griglia segue: alla chiusura il focus è sulla stessa foto.
            moveCursor(n, false);
          }}
          onClose={() => setViewerAt(null)}
          onKey={(key) => {
            const it = at_(viewerAt).item;
            const action = it ? hotkeyAction(key, activeSet(tagInfo)) : null;
            if (!it || !action) return false;
            void bulk.start(
              { kind: "ids", ids: [it.id] },
              action,
              action.action === "add_tag"
                ? t("selection.done.add_tag", { name: action.name, count: 1 })
                : t("selection.done.remove_tags", { count: 1 }),
            );
            return true;
          }}
        />
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
