import { Coachmark } from "../../components/guide/Coachmark";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type Correction, type FaceScope, type GroupRow } from "../../lib/ipc";
import { listAllGroups } from "./allGroups";
import { fmtDec, fmtInt } from "../../lib/format";
import { clickSelect, rectangleSelect } from "../../store/library";
import { QueryView } from "../../components/feedback/QueryView";
import { notify, notifyError } from "../../store/toasts";
import { FaceCrop } from "./FaceCrop";
import { MergeDialog } from "./MergeDialog";
import { discardReasonLabel } from "./discardReason";
import type { GroupListTarget } from "./GroupList";

const PAGE = 80;
const GAP = 10;
const PAD = 12;
const META = 22;
const CELL_MIN = 96;

type Box = { x: number; y: number; w: number; h: number };

export type FaceHighlight = {
  faceId: number;
  index: number;
  discarded: boolean;
};

export type FaceGridHandle = {
  toggleFocusedSelection: () => void;
  focusMoveSelect: () => void;
  runNotSame: () => boolean;
  scrollTop: () => number;
};

export const FaceGrid = forwardRef<
  FaceGridHandle,
  {
    target: GroupListTarget;
    group: GroupRow | null;
    singlesCount?: number;
    /** Volto da evidenziare (bordo warn) e portare in vista, da openPeople. */
    highlight?: FaceHighlight | null;
    onSelectedFacesChange?: (ids: number[]) => void;
  }
>(function FaceGrid(
  { target, group, singlesCount = 0, highlight = null, onSelectedFacesChange },
  ref,
) {
  const tipAnchor = useRef<HTMLHeadingElement>(null);
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [discardedOnly, setDiscardedOnly] = useState(false);
  const [selected, setSelected] = useState<number[]>([]);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [focusedFace, setFocusedFace] = useState<number | null>(null);
  const [moveTo, setMoveTo] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [mergeHintOpen, setMergeHintOpen] = useState(false);
  const [width, setWidth] = useState(640);
  const [box, setBox] = useState<Box | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const moveSelectRef = useRef<HTMLSelectElement>(null);
  const drag = useRef<{ x: number; y: number; initial: number[] } | null>(null);
  const scrolledFor = useRef<number | null>(null);

  useEffect(() => {
    setSelected([]);
    setAnchor(null);
    setFocusedFace(null);
    setMoveTo("");
    setDiscardedOnly(highlight?.discarded ?? false);
    setError("");
    scrolledFor.current = null;
    onSelectedFacesChange?.([]);
  }, [target?.kind === "group" ? target.id : target?.kind, group?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (highlight?.discarded) setDiscardedOnly(true);
  }, [highlight?.faceId, highlight?.discarded]);

  useEffect(() => {
    onSelectedFacesChange?.(selected);
  }, [selected]); // eslint-disable-line react-hooks/exhaustive-deps

  const scope: FaceScope | null = useMemo(() => {
    if (!target || target.kind === "identity") return null;
    if (target.kind === "singles") return { kind: "singles" };
    if (target.kind === "discarded") return { kind: "discarded", group: null };
    if (discardedOnly) return { kind: "discarded", group: target.id };
    return { kind: "group", group: target.id };
  }, [target, discardedOnly]);

  const faces = useInfiniteQuery({
    queryKey: ["group-faces", scope],
    queryFn: ({ pageParam }) => commands.groupFaces(scope!, "quality_desc", pageParam, PAGE),
    initialPageParam: null as { qualityScore: number | null; id: number } | null,
    getNextPageParam: (last) => last.next ?? undefined,
    enabled: isTauri() && scope != null,
    placeholderData: (prev) => prev,
  });

  const moveTargets = useQuery({
    queryKey: ["people-move-targets"],
    queryFn: async () => {
      const [review, people] = await Promise.all([
        listAllGroups("review", "photos_desc"),
        listAllGroups("people", "name_asc"),
      ]);
      return [...people, ...review];
    },
    enabled: isTauri() && target != null,
  });

  const items = useMemo(() => faces.data?.pages.flatMap((p) => p.items) ?? [], [faces.data]);
  const order = useMemo(() => items.map((i) => i.id), [items]);
  const meta = faces.data?.pages[0];
  const goodCount = meta?.goodCount ?? group?.goodFaces ?? 0;
  const discardedCount = meta?.discardedCount ?? group?.discardedFaces ?? 0;
  const photos = group?.photos ?? 0;

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([e]) => {
      const w = e.contentRect.width;
      if (w > 0) setWidth(w);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const cols = Math.max(1, Math.floor((width - PAD * 2 + GAP) / (CELL_MIN + GAP)));
  const cell = Math.max(CELL_MIN, Math.floor((width - PAD * 2 - (cols - 1) * GAP) / cols));
  const rowHeight = cell + META + GAP;
  const rows = Math.ceil(items.length / cols) || 0;

  const virt = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 4,
    initialRect: { width: width || 640, height: 600 },
    observeElementRect: (_instance, cb) => {
      const el = scrollRef.current;
      const emit = () => {
        const w = el?.clientWidth ?? 0;
        const h = el?.clientHeight ?? 0;
        cb({ width: w || width || 640, height: h || 600 });
      };
      emit();
      if (!el || typeof ResizeObserver === "undefined") return () => {};
      const ro = new ResizeObserver(emit);
      ro.observe(el);
      return () => ro.disconnect();
    },
  });

  useEffect(() => virt.measure(), [rowHeight, cols, virt]);

  const vitems = virt.getVirtualItems();
  const lastRow = vitems[vitems.length - 1]?.index ?? 0;
  useEffect(() => {
    if (lastRow >= rows - 3 && faces.hasNextPage && !faces.isFetchingNextPage)
      void faces.fetchNextPage();
  }, [lastRow, rows, faces]);

  // Carica pagine finché l'indice del volto evidenziato è in memoria, poi scorre.
  useEffect(() => {
    if (!highlight || faces.isLoading) return;
    if (items.length <= highlight.index && faces.hasNextPage && !faces.isFetchingNextPage) {
      void faces.fetchNextPage();
      return;
    }
    if (scrolledFor.current === highlight.faceId) return;
    if (!items.some((f) => f.id === highlight.faceId) && items.length <= highlight.index) return;
    const row = Math.floor(highlight.index / cols);
    scrolledFor.current = highlight.faceId;
    virt.scrollToIndex(row, { align: "center" });
  }, [highlight, items, cols, faces, virt]);

  const setSelection = (next: number[], nextAnchor?: number | null) => {
    setSelected(next);
    if (nextAnchor !== undefined) setAnchor(nextAnchor);
  };

  const onFaceClick = (id: number) => (e: MouseEvent) => {
    e.stopPropagation();
    setFocusedFace(id);
    const next = clickSelect(order, selected, anchor, id, {
      ctrl: e.ctrlKey || e.metaKey,
      shift: e.shiftKey,
    });
    setSelection(next, id);
  };

  const refresh = async () => {
    await qc.invalidateQueries({ queryKey: ["group-faces"] });
    await qc.invalidateQueries({ queryKey: ["people-groups"] });
    await qc.invalidateQueries({ queryKey: ["people-sections"] });
  };

  // "Non è la stessa persona": esito nell'unico ToastHost, con annullamento.
  const showNotSameUndo = () =>
    notify({
      key: "people-not-same",
      tone: "ok",
      message: t("people.notSameDone"),
      action: {
        label: t("people.undo"),
        run: async () => {
          try {
            await commands.undoPeople();
            await refresh();
          } catch (err) {
            notifyError(err);
          }
        },
      },
    });
  const correct = async (...corrections: Correction[]) => {
    if (!corrections.length) return;
    setBusy(true);
    setError("");
    try {
      for (const c of corrections) await commands.correctPerson(c);
      setSelected([]);
      setMoveTo("");
      await refresh();
    } catch (err) {
      setError((err as { detail?: string }).detail ?? String(err));
    } finally {
      setBusy(false);
    }
  };

  useImperativeHandle(ref, () => ({
    toggleFocusedSelection() {
      const id = focusedFace ?? items[0]?.id;
      if (id == null) return;
      setFocusedFace(id);
      const next = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
      setSelection(next, id);
    },
    focusMoveSelect() {
      moveSelectRef.current?.focus();
    },
    runNotSame() {
      const hint = group?.mergeHint;
      const gid = target?.kind === "group" ? target.id : null;
      if (selected.length && gid != null) {
        void correct(
          ...selected.map((face): Correction => ({ action: "reject", person: gid, face })),
        );
        return true;
      }
      if (!hint || gid == null || busy) return false;
      void (async () => {
        setBusy(true);
        setError("");
        try {
          await commands.correctPerson({
            action: "not_same",
            a: gid,
            b: hint.otherId,
          });
          await refresh();
          showNotSameUndo();
        } catch (err) {
          setError((err as { detail?: string }).detail ?? String(err));
        } finally {
          setBusy(false);
        }
      })();
      return true;
    },
    scrollTop: () => scrollRef.current?.scrollTop ?? 0,
  }));

  const groupId = target?.kind === "group" ? target.id : null;
  const title =
    target?.kind === "singles"
      ? t("people.singlesLabel", { count: singlesCount })
      : target?.kind === "discarded"
        ? t("people.discardedAllTitle")
        : group?.name
          ? group.name
          : group
            ? t("people.groupOfFaces", { count: group.faces })
            : t("people.facesLabel");

  const moveOptions = useMemo(() => {
    const seen = new Set<number>();
    const out: GroupRow[] = [];
    for (const g of moveTargets.data ?? []) {
      if (g.id === groupId || seen.has(g.id)) continue;
      seen.add(g.id);
      out.push(g);
    }
    return out;
  }, [moveTargets.data, groupId]);

  if (!target) {
    return (
      <div style={{ padding: 16 }}>
        <p className="muted" style={{ margin: 0 }}>
          {t("people.pickGroup")}
        </p>
      </div>
    );
  }

  const hint = group?.mergeHint;

  return (
    <div
      style={{
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "12px 16px",
          display: "flex",
          flexWrap: "wrap",
          gap: 8,
          alignItems: "center",
          borderBottom: "1px solid var(--pc-border)",
        }}
      >
        <h1 ref={tipAnchor} style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>
          {title}
        </h1>
        <Coachmark tip="people" anchor={tipAnchor} learnMore="people" />
        {target.kind === "group" && (
          <>
            <span className="chip mono">{t("people.photos", { count: photos })}</span>
            <span className="chip" style={{ color: "var(--pc-ok, inherit)" }}>
              {t("people.goodQuality", { count: goodCount })}
            </span>
            <span className="chip" style={{ color: "var(--pc-warn, inherit)" }}>
              {t("people.discardedQuality", { count: discardedCount })}
            </span>
          </>
        )}
      </div>

      <div
        role="toolbar"
        aria-label={t("people.facesToolbar")}
        style={{
          padding: "8px 16px",
          display: "flex",
          flexWrap: "wrap",
          gap: 8,
          alignItems: "center",
          borderBottom: "1px solid var(--pc-border)",
        }}
      >
        {selected.length > 0 && (
          <span className="muted">{t("people.selected", { count: selected.length })}</span>
        )}
        <select
          ref={moveSelectRef}
          className="field"
          aria-label={t("people.move")}
          value={moveTo}
          disabled={busy || selected.length === 0}
          onChange={(e) => {
            const v = e.target.value;
            setMoveTo(v);
            if (v && selected.length)
              void correct({ action: "move", faces: selected, to: Number(v) });
          }}
          style={{ maxWidth: 180 }}
        >
          <option value="">{t("people.move")}</option>
          {moveOptions.map((g) => (
            <option key={g.id} value={g.id}>
              {g.name ?? t("people.groupOfFaces", { count: g.faces })}
            </option>
          ))}
        </select>
        {groupId != null && (
          <>
            <button
              type="button"
              className="btn"
              disabled={busy || selected.length === 0}
              onClick={() =>
                void correct(
                  ...selected.map((face): Correction => ({
                    action: "reject",
                    person: groupId,
                    face,
                  })),
                )
              }
            >
              {t("people.reject")}
            </button>
            <button
              type="button"
              className="btn"
              disabled={busy || selected.length === 0}
              onClick={() => void correct({ action: "separate", person: groupId, faces: selected })}
            >
              {t("people.newGroup")}
            </button>
          </>
        )}
        <button
          type="button"
          className="btn"
          disabled={busy || selected.length === 0}
          onClick={() => {
            if (!window.confirm(t("people.removeConfirm", { count: selected.length }))) return;
            void correct(...selected.map((face): Correction => ({ action: "remove", face })));
          }}
          style={{ color: "var(--pc-danger, inherit)" }}
        >
          {t("people.remove")}
        </button>
        {discardedOnly && (
          <button
            type="button"
            className="btn"
            disabled={busy || selected.length === 0}
            onClick={() => void correct({ action: "recover", faces: selected })}
          >
            {t("people.recover")}
          </button>
        )}
        <span aria-hidden style={{ width: 1, height: 20, background: "var(--pc-border)" }} />
        <span className="muted">{t("people.showLabel")}</span>
        <button
          type="button"
          className="btn"
          aria-pressed={!discardedOnly}
          onClick={() => setDiscardedOnly(false)}
          style={{
            background: !discardedOnly ? "var(--pc-raised)" : "transparent",
            borderColor: !discardedOnly ? "var(--pc-border)" : "transparent",
          }}
        >
          {t("people.showAll")}
        </button>
        <button
          type="button"
          className="btn"
          aria-pressed={discardedOnly}
          onClick={() => setDiscardedOnly(true)}
          disabled={target.kind !== "group"}
          style={{
            background: discardedOnly ? "var(--pc-raised)" : "transparent",
            borderColor: discardedOnly ? "var(--pc-border)" : "transparent",
          }}
        >
          {t("people.showDiscarded")}
        </button>
      </div>

      {hint && groupId != null && !discardedOnly && (
        <div
          role="note"
          style={{
            margin: "8px 16px 0",
            padding: 12,
            display: "flex",
            flexWrap: "wrap",
            gap: 12,
            alignItems: "center",
            background: "var(--pc-raised)",
            border: "1px solid var(--pc-border)",
            borderRadius: 6,
          }}
        >
          <div style={{ flex: "1 1 220px" }}>
            <b>
              {t("people.mergeSuggestTitle", {
                name: hint.otherName ?? t("people.groupOfFaces", { count: hint.otherFaces }),
              })}
            </b>
            <br />
            <span className="muted">
              {hint.score != null
                ? t("people.mergeSuggestBody", { score: fmtDec(hint.score, 2) })
                : t("people.mergeSuggestBodyNoScore")}
            </span>
          </div>
          <button
            type="button"
            className="btn primary"
            disabled={busy}
            onClick={() => setMergeHintOpen(true)}
          >
            {t("people.samePerson")}
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => {
              void (async () => {
                setBusy(true);
                setError("");
                try {
                  await commands.correctPerson({
                    action: "not_same",
                    a: groupId,
                    b: hint.otherId,
                  });
                  await refresh();
                  showNotSameUndo();
                } catch (err) {
                  setError((err as { detail?: string }).detail ?? String(err));
                } finally {
                  setBusy(false);
                }
              })();
            }}
          >
            {t("people.notSame")}
          </button>
        </div>
      )}

      {error && (
        <p role="alert" style={{ margin: "8px 16px 0" }}>
          {error}
        </p>
      )}

      {mergeHintOpen && group && hint && (
        <MergeDialog
          sources={[
            {
              ...group,
              id: hint.otherId,
              name: hint.otherName,
              faces: hint.otherFaces,
              photos: hint.otherPhotos,
              cover: hint.otherCover,
            },
          ]}
          target={group}
          onClose={() => setMergeHintOpen(false)}
          onMerged={() => void refresh()}
        />
      )}

      <div
        ref={scrollRef}
        data-testid="face-grid-scroll"
        role="grid"
        aria-label={t("people.facesGrid")}
        style={{ flex: 1, overflow: "auto", position: "relative", minHeight: 0 }}
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
          const el = e.currentTarget;
          const r = el.getBoundingClientRect();
          if (e.clientY > r.bottom - 30) el.scrollTop += 16;
          else if (e.clientY < r.top + 30) el.scrollTop -= 16;
          const x = e.clientX - r.left;
          const y = e.clientY - r.top + el.scrollTop;
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
              ...rectangleSelect(order, b, {
                cols,
                cell,
                rowHeight,
                gap: GAP,
                pad: PAD,
              }),
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
        <QueryView
          query={faces}
          isEmpty={() => items.length === 0}
          skeleton={
            <p className="muted" style={{ padding: 16 }}>
              {t("people.loading")}
            </p>
          }
          error={t("people.failed")}
          empty={
            <p className="muted" style={{ padding: 16 }}>
              {discardedOnly ? t("people.emptyDiscarded") : t("people.emptyFaces")}
            </p>
          }
        >
          {() => (
            <>
              <div style={{ height: virt.getTotalSize() + PAD * 2, position: "relative" }}>
                {box && (
                  <div
                    aria-hidden
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
                    {items.slice(row.index * cols, row.index * cols + cols).map((face) => {
                      const sel = selected.includes(face.id);
                      const lit = highlight?.faceId === face.id;
                      const tip = discardReasonLabel(face.discardReason, face.sizePx, t);
                      return (
                        <button
                          key={face.id}
                          type="button"
                          data-testid={`face-${face.id}`}
                          data-face-tile
                          data-highlighted={lit || undefined}
                          aria-pressed={sel}
                          title={tip}
                          onClick={onFaceClick(face.id)}
                          style={{
                            width: cell,
                            padding: 0,
                            margin: 0,
                            borderRadius: 6,
                            overflow: "hidden",
                            cursor: "pointer",
                            textAlign: "left",
                            background: "var(--pc-panel)",
                            border: `2px solid ${
                              lit ? "var(--pc-warn)" : sel ? "var(--pc-accent)" : "var(--pc-raised)"
                            }`,
                            color: "inherit",
                            font: "inherit",
                            opacity: face.discardReason ? 0.72 : 1,
                          }}
                        >
                          <FaceCrop face={face} size={cell - 4} />
                          <span
                            style={{
                              display: "flex",
                              justifyContent: "space-between",
                              padding: "2px 6px",
                              fontSize: 11,
                              color: "var(--pc-text-soft)",
                            }}
                          >
                            <span className="mono">
                              {face.year != null ? String(face.year) : ""}
                            </span>
                            {face.qualityScore != null && (
                              <span className="mono">{fmtDec(face.qualityScore, 2)}</span>
                            )}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                ))}
              </div>
            </>
          )}
        </QueryView>
      </div>

      <div
        className="muted"
        style={{
          padding: "6px 16px",
          borderTop: "1px solid var(--pc-border)",
          fontSize: 12,
          display: "flex",
          gap: 16,
          flexWrap: "wrap",
        }}
      >
        <span>{t("people.facesSortHint")}</span>
        <span className="mono">
          {fmtInt(items.length)}
          {meta?.total ? ` / ${fmtInt(meta.total)}` : ""}
        </span>
      </div>
    </div>
  );
});
