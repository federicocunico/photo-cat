import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
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
import { commands, isTauri, type GridItem, type Page } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import {
  clickSelect,
  rectangleSelect,
  emptyFilter,
  thumbPx,
  useLibrary,
} from "../../store/library";
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

export const PAGE_SIZE = 300;
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
}: {
  item: GridItem;
  size: number;
  selected: boolean;
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
      title={item.name}
      style={{
        width: size,
        padding: 0,
        margin: 0,
        borderRadius: 6,
        overflow: "hidden",
        cursor: "pointer",
        textAlign: "left",
        background: "var(--pc-panel)",
        border: `2px solid ${selected ? "var(--pc-accent)" : "var(--pc-raised)"}`,
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

/** Griglia virtualizzata: carica le pagine (keyset) man mano che si scorre. */
export function LibraryGrid({ initialWidth }: { initialWidth?: number }) {
  const { t } = useTranslation();
  const { filter, sort, thumbSize, selected, focused, setSelection, scrollTop, setScrollTop } =
    useLibrary();
  const size = thumbPx(thumbSize);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(initialWidth ?? 800);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const drag = useRef<{ x: number; y: number; initial: number[] } | null>(null);
  const tagInfo = useTagOverview().data;
  const tagColor = (name: string) => colorOf(tagInfo, name);
  const [menu, setMenu] = useState<{ x: number; y: number; ids: number[] } | null>(null);
  const openWith = useOpenWith();
  const place = usePlaceAction();
  const closeMenu = useCallback(() => setMenu(null), []);
  const actionError = openWith.error || place.error;

  const pages = useInfiniteQuery({
    queryKey: ["library", filter, sort],
    queryFn: ({ pageParam }) => commands.libraryPage(filter, sort, pageParam, PAGE_SIZE),
    initialPageParam: null as Page["next"],
    getNextPageParam: (last) => last.next ?? undefined,
    enabled: isTauri(),
  });
  const total = useQuery({
    queryKey: ["library-count", filter],
    queryFn: () => commands.libraryCount(filter),
    enabled: isTauri(),
  });
  const items = useMemo(() => pages.data?.pages.flatMap((p) => p.items) ?? [], [pages.data]);
  const order = useMemo(() => items.map((i) => i.id), [items]);

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
  const rows = Math.ceil(items.length / cols);
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
  const lastRow = vitems[vitems.length - 1]?.index ?? 0;
  useEffect(() => {
    if (lastRow >= rows - 3 && pages.hasNextPage && !pages.isFetchingNextPage)
      void pages.fetchNextPage();
  }, [lastRow, rows, pages]);

  const onClick = (id: number) => (e: MouseEvent) => {
    const next = clickSelect(order, selected, focused, id, {
      ctrl: e.ctrlKey || e.metaKey,
      shift: e.shiftKey,
    });
    setSelection(next, id);
  };
  // Clic destro: sulla selezione agisce su tutta, altrimenti seleziona solo la miniatura.
  const onMenu = (id: number) => (x: number, y: number) => {
    const ids = selected.includes(id) ? selected : [id];
    if (!selected.includes(id)) setSelection([id], id);
    setMenu({ x, y, ids });
  };

  if (pages.isSuccess && items.length === 0) {
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
      data-testid="grid-scroll"
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
      style={{ flex: 1, overflow: "auto", position: "relative" }}
      aria-label={t("library.count", { n: fmtInt(total.data ?? items.length) })}
      role="region"
      onPointerDown={(e) => {
        if (e.button !== 0 || ((e.target as HTMLElement).closest("button") && !e.altKey)) return;
        const r = e.currentTarget.getBoundingClientRect();
        drag.current = {
          x: e.clientX - r.left,
          y: e.clientY - r.top + e.currentTarget.scrollTop,
          initial: e.ctrlKey || e.metaKey ? selected : [],
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
        setSelection([
          ...new Set([
            ...d.initial,
            ...rectangleSelect(order, b, { cols, cell, rowHeight, gap: GAP, pad: PAD }),
          ]),
        ]);
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
        {vitems.map((row) => (
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
            {items.slice(row.index * cols, row.index * cols + cols).map((it) => (
              <Thumb
                key={it.id}
                item={it}
                size={cell}
                selected={selected.includes(it.id)}
                onClick={onClick(it.id)}
                onDoubleClick={() => void openWith.open([it.id], { kind: "double_click" })}
                onMenu={onMenu(it.id)}
                tagColor={tagColor}
              />
            ))}
          </div>
        ))}
      </div>
      {pages.isFetchingNextPage && (
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
          onOpen={(ids, target) => void openWith.open(ids, target)}
          onPlace={(ids, p) => void place.apply(ids, p)}
          onClose={closeMenu}
        />
      )}
    </div>
  );
}
