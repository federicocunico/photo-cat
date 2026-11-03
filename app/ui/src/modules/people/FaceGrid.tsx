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
import { Skeleton } from "../../components/feedback/Skeleton";
import { Button, ConfirmDialog, InlineAlert } from "../../components/ui";
import { notifyPeopleUndo, refreshPeople } from "./peopleUndo";
import "./people.css";
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
  const [removeOpen, setRemoveOpen] = useState(false);
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

  const refresh = () => refreshPeople(qc);

  /**
   * Applica le correzioni in ordine. Con `done` l'esito compare come avviso con
   * "Annulla operazione" (e Ctrl+Z): ogni correzione è una voce del registro del backend.
   */
  const correct = async (corrections: Correction[], done?: string) => {
    if (!corrections.length) return;
    setBusy(true);
    setError("");
    try {
      for (const c of corrections) await commands.correctPerson(c);
      setSelected([]);
      setMoveTo("");
      await refresh();
      if (done) notifyPeopleUndo(qc, done, corrections.length);
    } catch (err) {
      setError((err as { detail?: string }).detail ?? String(err));
    } finally {
      setBusy(false);
    }
  };

  const reject = (gid: number) =>
    correct(
      selected.map((face): Correction => ({ action: "reject", person: gid, face })),
      t("people.rejectedDone", { count: selected.length }),
    );

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
        void reject(gid);
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
          notifyPeopleUndo(qc, t("people.notSameDone"));
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
            ? t("people.unnamed", { count: group.photos })
            : t("people.facesLabel");

  const labelOf = (g: GroupRow) => g.name ?? t("people.unnamed", { count: g.photos });

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

  const moveTarget = moveOptions.find((g) => String(g.id) === moveTo) ?? null;

  if (!target) {
    return (
      <div className="people-pad">
        <p className="muted people-muted-reset">{t("people.pickGroup")}</p>
      </div>
    );
  }

  const hint = group?.mergeHint;

  return (
    <div className="people-grid">
      <div className="people-grid-head">
        <h2 ref={tipAnchor} className="people-title">
          {title}
        </h2>
        <Coachmark tip="people" anchor={tipAnchor} learnMore="people" />
        {target.kind === "group" && (
          <>
            <span className="chip mono">{t("people.photos", { count: photos })}</span>
            <span className="chip people-chip-ok">
              {t("people.goodQuality", { count: goodCount })}
            </span>
            <span className="chip people-chip-warn">
              {t("people.discardedQuality", { count: discardedCount })}
            </span>
          </>
        )}
      </div>

      <div role="toolbar" aria-label={t("people.facesToolbar")} className="people-toolbar">
        {selected.length > 0 && (
          <span className="muted">{t("people.selected", { count: selected.length })}</span>
        )}
        {/* "Sposta in…": la scelta da sola non sposta nulla (su Windows le frecce cambiano
            una select chiusa); i volti si spostano solo con il pulsante "Sposta". */}
        <select
          ref={moveSelectRef}
          className="field people-move-select"
          aria-label={t("people.move")}
          value={moveTo}
          disabled={busy || selected.length === 0}
          onChange={(e) => setMoveTo(e.target.value)}
        >
          <option value="">{t("people.move")}</option>
          {moveOptions.map((g) => (
            <option key={g.id} value={g.id}>
              {labelOf(g)}
            </option>
          ))}
        </select>
        <Button
          disabled={busy}
          disabledReason={
            selected.length === 0
              ? t("people.noSelection")
              : !moveTarget
                ? t("people.moveDisabled")
                : undefined
          }
          onClick={() =>
            moveTarget &&
            correct(
              [{ action: "move", faces: selected, to: moveTarget.id }],
              t("people.movedDone", { count: selected.length, name: labelOf(moveTarget) }),
            )
          }
        >
          {t("people.moveApply")}
        </Button>
        {groupId != null && (
          <>
            <Button disabled={busy || selected.length === 0} onClick={() => reject(groupId)}>
              {t("people.reject")}
            </Button>
            <Button
              disabled={busy || selected.length === 0}
              onClick={() =>
                correct(
                  [{ action: "separate", person: groupId, faces: selected }],
                  t("people.separatedDone", { count: selected.length }),
                )
              }
            >
              {t("people.newGroup")}
            </Button>
          </>
        )}
        <Button
          variant="danger"
          disabled={busy || selected.length === 0}
          onClick={() => setRemoveOpen(true)}
        >
          {t("people.remove")}
        </Button>
        {discardedOnly && (
          <Button
            disabled={busy || selected.length === 0}
            onClick={() =>
              correct(
                [{ action: "recover", faces: selected }],
                t("people.recoveredDone", { count: selected.length }),
              )
            }
          >
            {t("people.recover")}
          </Button>
        )}
        <span aria-hidden className="people-toolbar-sep" />
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
        <div role="note" className="people-merge-hint">
          <div className="people-merge-hint-body">
            <b>
              {t("people.mergeSuggestTitle", {
                name: hint.otherName ?? t("people.unnamedPlain"),
              })}
            </b>
            <br />
            <span className="muted">
              {hint.score != null
                ? t("people.mergeSuggestBody", { score: fmtDec(hint.score, 2) })
                : t("people.mergeSuggestBodyNoScore")}
            </span>
          </div>
          <Button variant="primary" disabled={busy} onClick={() => setMergeHintOpen(true)}>
            {t("people.samePerson")}
          </Button>
          <Button
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
                  notifyPeopleUndo(qc, t("people.notSameDone"));
                } catch (err) {
                  setError((err as { detail?: string }).detail ?? String(err));
                } finally {
                  setBusy(false);
                }
              })();
            }}
          >
            {t("people.notSame")}
          </Button>
        </div>
      )}

      {error && (
        <div className="people-alert">
          <InlineAlert tone="danger" title={t("people.failed")} onDismiss={() => setError("")}>
            {error}
          </InlineAlert>
        </div>
      )}

      <ConfirmDialog
        open={removeOpen}
        onOpenChange={setRemoveOpen}
        title={t("people.removeTitle", { count: selected.length })}
        message={t("people.removeBody")}
        confirmLabel={t("people.removeButton", { count: selected.length })}
        onConfirm={() => correct(selected.map((face): Correction => ({ action: "remove", face })))}
      />

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
        role="group"
        aria-label={t("people.facesGrid")}
        className="people-grid-scroll"
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
          skeleton={<Skeleton kind="thumbnail" count={8} />}
          error={t("people.failed")}
          empty={
            <p className="muted people-empty-pad">
              {discardedOnly ? t("people.emptyDiscarded") : t("people.emptyFaces")}
            </p>
          }
        >
          {() => (
            <>
              <div className="people-virt-pad" style={{ height: virt.getTotalSize() + PAD * 2 }}>
                {box && (
                  <div
                    aria-hidden
                    className="people-marquee"
                    style={{
                      left: box.x,
                      top: box.y,
                      width: box.w,
                      height: box.h,
                    }}
                  />
                )}
                {vitems.map((row) => (
                  <div key={row.key} className="people-virt-cells" style={{ top: row.start + PAD }}>
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
                          className="people-face-tile"
                          style={{
                            width: cell,
                            border: `2px solid ${
                              lit ? "var(--pc-warn)" : sel ? "var(--pc-accent)" : "var(--pc-raised)"
                            }`,
                            opacity: face.discardReason ? 0.72 : 1,
                          }}
                        >
                          <FaceCrop face={face} size={cell - 4} />
                          {tip && <span className="visually-hidden">{tip}</span>}
                          <span className="people-face-meta">
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

      <div className="muted people-grid-foot">
        <span>{t("people.facesSortHint")}</span>
        <span className="mono">
          {fmtInt(items.length)}
          {meta?.total ? ` / ${fmtInt(meta.total)}` : ""}
        </span>
      </div>
    </div>
  );
});
