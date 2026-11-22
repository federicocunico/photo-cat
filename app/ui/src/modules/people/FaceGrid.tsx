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
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Correction,
  type FaceScope,
  type FaceSort,
  type FaceView,
  type GroupRow,
  type OpenTarget,
} from "../../lib/ipc";
import { FACE_SORTS, usePeoplePrefs } from "../../store/peoplePrefs";
import { buildFaceRows, orderYears, rectangleSelectRows } from "./faceLayout";
import { listAllGroups } from "./allGroups";
import { fmtDec, fmtInt } from "../../lib/format";
import { idsSelection, rectangleSelect, selSet, type Selection } from "../../store/library";
import { useGridSelection } from "../../lib/useGridSelection";
import { useActiveGrid } from "../../commands/gridShortcuts";
import { QueryView } from "../../components/feedback/QueryView";
import { Skeleton } from "../../components/feedback/Skeleton";
import {
  Button,
  ConfirmDialog,
  ContextMenu,
  InlineAlert,
  type MenuItem,
} from "../../components/ui";
import { notifyPeopleUndo, refreshPeople, refreshPeopleFast } from "./peopleUndo";
import "./people.css";
import { FaceCrop } from "./FaceCrop";
import { MergeDialog } from "./MergeDialog";
import { discardReasonLabel } from "./discardReason";
import { Spinner } from "../../components/feedback/Spinner";
import type { GroupListTarget } from "./GroupList";
import { FaceViewer } from "./FaceViewer";
import { distinctByMedia, useFacePhotoActions } from "./useFacePhotoActions";

const PAGE = 80;
const GAP = 10;
const PAD = 12;
const META = 22;
const CELL_MIN = 96;
const HEAD = 32;

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

/** Etichetta con il numero di volti su cui agisce, quando sono più di uno (issue #30). */
const withCount = (label: string, n: number) => (n > 1 ? `${label} (${n})` : label);

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
  const [focusedFace, setFocusedFace] = useState<number | null>(null);
  const [moveTo, setMoveTo] = useState("");
  const [busy, setBusy] = useState(false);
  // Avanzamento delle correzioni su più volti (issue #28): "3 di 40".
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [error, setError] = useState("");
  const [mergeHintOpen, setMergeHintOpen] = useState(false);
  const [removeOpen, setRemoveOpen] = useState(false);
  const [width, setWidth] = useState(640);
  const [box, setBox] = useState<Box | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const moveSelectRef = useRef<HTMLSelectElement>(null);
  const drag = useRef<{ x: number; y: number; initial: Selection<FaceScope> | null } | null>(null);
  const scrolledFor = useRef<number | null>(null);
  const photoActions = useFacePhotoActions();
  // Volto su cui si è aperto il menu contestuale e visore aperto (volti e posizione).
  const [menuFace, setMenuFace] = useState<number | null>(null);
  const [viewer, setViewer] = useState<{ faces: FaceView[]; index: number } | null>(null);

  useEffect(() => {
    setFocusedFace(null);
    setMoveTo("");
    setDiscardedOnly(highlight?.discarded ?? false);
    setError("");
    scrolledFor.current = null;
  }, [target?.kind === "group" ? target.id : target?.kind, group?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (highlight?.discarded) setDiscardedOnly(true);
  }, [highlight?.faceId, highlight?.discarded]);

  const scope: FaceScope | null = useMemo(() => {
    if (!target || target.kind === "identity") return null;
    if (target.kind === "singles") return { kind: "singles" };
    if (target.kind === "discarded") return { kind: "discarded", group: null };
    if (discardedOnly) return { kind: "discarded", group: target.id };
    return { kind: "group", group: target.id };
  }, [target, discardedOnly]);

  const sort = usePeoplePrefs((p) => p.faceSort);
  const setSort = usePeoplePrefs((p) => p.setFaceSort);
  const byDate = sort !== "quality_desc";

  const faces = useInfiniteQuery({
    queryKey: ["group-faces", scope, sort],
    queryFn: ({ pageParam }) => commands.groupFaces(scope!, sort, pageParam, PAGE),
    initialPageParam: null as { qualityScore: number | null; id: number } | null,
    getNextPageParam: (last) => last.next ?? undefined,
    enabled: isTauri() && scope != null,
    placeholderData: (prev) => prev,
  });

  // Anni con i conteggi del backend: le intestazioni valgono anche per le pagine non caricate.
  const yearsQ = useQuery({
    queryKey: ["group-face-years", scope],
    queryFn: () => commands.groupFaceYears(scope!),
    enabled: isTauri() && scope != null && byDate,
    placeholderData: (prev) => prev,
  });

  // «Riassegna a…»: tutte le persone con un nome, comprese le identità che in questa
  // libreria non hanno ancora volti (la persona viene creata alla riassegnazione).
  const config = useQuery({
    queryKey: ["config"],
    queryFn: commands.getConfig,
    enabled: isTauri(),
  });

  const moveTargets = useQuery({
    queryKey: ["people-move-targets"],
    queryFn: async () => {
      const [people, ids] = await Promise.all([
        listAllGroups("people", "name_asc"),
        commands.listIdentities(null, null),
      ]);
      return { people, identities: ids.identities.filter((i) => i.person == null) };
    },
    enabled: isTauri() && target != null,
  });

  const items = useMemo(() => faces.data?.pages.flatMap((p) => p.items) ?? [], [faces.data]);
  const order = useMemo(() => items.map((i) => i.id), [items]);
  const sel = useGridSelection<FaceScope>({
    // Il filtro è lo scope: cambiando gruppo o vista la selezione ricomincia (vedi sotto).
    filter: scope ?? { kind: "singles" },
    order,
    fetchBase: () => commands.faceSelection(scope!),
    onError: (e) => setError((e as { detail?: string }).detail ?? String(e)),
  });
  const selection = sel.selection;
  /** Id espliciti (vuoto con una selezione per filtro: si risolvono al momento dell'azione). */
  const selected = sel.ids ?? [];
  const selectedCount = sel.count;
  /**
   * Id dei volti selezionati: espliciti, oppure risolti dal backend per una selezione "tutti i
   * volti dello scope" (meno le eccezioni), anche se solo una parte è caricata.
   */
  const resolveFaceIds = async (): Promise<number[]> => {
    if (selection.mode === "ids") return [...selection.ids];
    return commands.faceSelectionIds(selection.filter, selection.maxId, [...selection.except]);
  };
  const scopeKey = JSON.stringify(scope);
  useEffect(() => {
    sel.clear();
  }, [scopeKey]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    onSelectedFacesChange?.(selected);
  }, [selection]); // eslint-disable-line react-hooks/exhaustive-deps
  useActiveGrid(
    {
      selectAll: sel.selectAll,
      invert: sel.invert,
      clear: sel.clear,
      count: () => selectedCount,
    },
    target != null && scope != null,
  );
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
    // Senza gruppo scelto la griglia non è montata: si osserva quando compare.
  }, [target == null]); // eslint-disable-line react-hooks/exhaustive-deps

  const cols = Math.max(1, Math.floor((width - PAD * 2 + GAP) / (CELL_MIN + GAP)));
  const cell = Math.max(CELL_MIN, Math.floor((width - PAD * 2 - (cols - 1) * GAP) / cols));
  const rowHeight = cell + META + GAP;
  // Con l'ordinamento per data le righe seguono i conteggi per anno (intestazioni incluse);
  // se i conteggi non coincidono con il totale (dati in aggiornamento) si resta senza intestazioni.
  const faceRows = useMemo(() => {
    if (!byDate || !yearsQ.data || !meta) return null;
    const years = orderYears(yearsQ.data, sort as "captured_asc" | "captured_desc");
    if (years.reduce((n, y) => n + y.count, 0) !== meta.total) return null;
    return buildFaceRows(years, cols);
  }, [byDate, yearsQ.data, meta, sort, cols]);
  const rows = faceRows ? faceRows.length : Math.ceil(items.length / cols) || 0;

  const virt = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (faceRows?.[i]?.kind === "head" ? HEAD : rowHeight),
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

  useEffect(() => virt.measure(), [rowHeight, cols, virt, faceRows]);

  const vitems = virt.getVirtualItems();
  const lastRow = vitems[vitems.length - 1]?.index ?? 0;
  // Con le intestazioni le righe coprono anche i volti non caricati: si carica
  // finché la riga più bassa visibile (più un margine) è oltre l'ultimo volto in memoria.
  const reach = faceRows
    ? Math.max(0, ...faceRows.slice(0, lastRow + 4).map((r) => (r.kind === "faces" ? r.to : 0)))
    : 0;
  useEffect(() => {
    const needMore = faceRows ? reach > items.length - cols : lastRow >= rows - 3;
    if (needMore && faces.hasNextPage && !faces.isFetchingNextPage) void faces.fetchNextPage();
  }, [lastRow, rows, faces, faceRows, reach, items.length, cols]);

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

  const onFaceClick = (id: number) => (e: MouseEvent) => {
    e.stopPropagation();
    setFocusedFace(id);
    sel.click(id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
  };

  const refresh = () => refreshPeople(qc);

  /**
   * Applica le correzioni in ordine. Con `done` l'esito compare come avviso con
   * "Annulla operazione" (e Ctrl+Z): ogni correzione è una voce del registro del backend.
   */
  const correct = async (
    build: Correction[] | ((ids: number[]) => Correction[]),
    done?: string | ((count: number) => string),
    /** Id espliciti (menu su un volto fuori dalla selezione); altrimenti la selezione. */
    explicit?: number[],
  ) => {
    setBusy(true);
    setError("");
    try {
      let ids: number[] = [];
      if (typeof build === "function") ids = explicit ?? (await resolveFaceIds());
      const corrections = typeof build === "function" ? build(ids) : build;
      if (!corrections.length) return;
      setProgress(corrections.length > 1 ? { done: 0, total: corrections.length } : null);
      for (const [i, c] of corrections.entries()) {
        await commands.correctPerson(c);
        if (corrections.length > 1) setProgress({ done: i + 1, total: corrections.length });
      }
      sel.clear();
      setMoveTo("");
      if (done)
        notifyPeopleUndo(
          qc,
          typeof done === "function" ? done(ids.length) : done,
          corrections.length,
        );
      // I comandi tornano disponibili appena i volti del gruppo sono aggiornati.
      await refreshPeopleFast(qc);
    } catch (err) {
      setError((err as { detail?: string }).detail ?? String(err));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  const reject = (gid: number, explicit?: number[]) =>
    correct(
      (ids) => [{ action: "reject", person: gid, faces: ids }],
      (n) => t("people.rejectedDone", { count: n }),
      explicit,
    );

  useImperativeHandle(ref, () => ({
    toggleFocusedSelection() {
      const id = focusedFace ?? items[0]?.id;
      if (id == null) return;
      setFocusedFace(id);
      sel.click(id, { ctrl: true, shift: false });
    },
    focusMoveSelect() {
      moveSelectRef.current?.focus();
    },
    runNotSame() {
      const hint = group?.mergeHint;
      const gid = target?.kind === "group" ? target.id : null;
      if (selectedCount > 0 && gid != null) {
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
          notifyPeopleUndo(qc, t("people.notSameDone"));
          await refreshPeopleFast(qc);
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

  /** Il menu agisce sulla selezione se contiene il volto cliccato, altrimenti su quel solo volto. */
  const menuTargets = (): FaceView[] => {
    const f = items.find((i) => i.id === menuFace);
    if (!f) return [];
    return sel.has(f.id) ? items.filter((i) => sel.has(i.id)) : [f];
  };
  const menuItems = (): MenuItem[] => {
    const faces = menuTargets();
    if (!faces.length) return [{ kind: "label", label: t("people.emptyFaces") }];
    // Sulla selezione se contiene il volto cliccato (anche per filtro, oltre i caricati),
    // altrimenti su quel solo volto.
    const inSel = menuFace != null && sel.has(menuFace);
    const explicit = inSel ? undefined : faces.map((f) => f.id);
    const count = inSel ? selectedCount : 1;
    const photos = distinctByMedia(faces);
    const first = faces.find((f) => f.id === menuFace) ?? faces[0];
    // Selezione per filtro: le foto si risolvono nel backend, anche per i volti non caricati.
    const openTargets = (target: OpenTarget) =>
      inSel && selection.mode === "filter"
        ? commands
            .faceSelectionMedia(selection.filter, selection.maxId, [...selection.except])
            .then((m) => photoActions.openResolved(m, target))
            .catch((e) => setError((e as { detail?: string }).detail ?? String(e)))
        : photoActions.open(faces, target);
    const out: MenuItem[] = [
      {
        label: t("faceMenu.openPhoto"),
        onSelect: () => setViewer({ faces: photos, index: Math.max(0, photos.indexOf(first)) }),
      },
      { label: t("faceMenu.showInLibrary"), onSelect: () => void photoActions.library(first) },
      {
        kind: "submenu",
        label: t("externalApps.openWith"),
        items: [
          ...(config.data?.external_apps ?? []).map((a): MenuItem => ({
            label: a.name,
            onSelect: () => void openTargets({ kind: "app", id: a.id ?? "" }),
          })),
          {
            label: t("externalApps.system"),
            onSelect: () => void openTargets({ kind: "system" }),
          },
        ],
      },
      {
        label: t("externalApps.reveal"),
        onSelect: () => void openTargets({ kind: "reveal" }),
      },
      { kind: "separator" },
    ];
    if (groupId != null)
      out.push(
        {
          label: withCount(t("people.reject"), count),
          onSelect: () => void reject(groupId, explicit),
        },
        {
          label: t("people.newGroup"),
          onSelect: () =>
            correct(
              (ids) => [{ action: "separate", person: groupId, faces: ids }],
              (n) => t("people.separatedDone", { count: n }),
              explicit,
            ),
        },
      );
    if (discardedOnly)
      out.push({
        label: t("people.recover"),
        onSelect: () =>
          correct(
            (ids) => [{ action: "recover", faces: ids }],
            (n) => t("people.recoveredDone", { count: n }),
            explicit,
          ),
      });
    out.push({
      label: withCount(t("people.remove"), count),
      danger: true,
      onSelect: () => {
        if (explicit) sel.setIds(explicit);
        setRemoveOpen(true);
      },
    });
    out.push(
      { kind: "separator" },
      { label: t("people.selectAll"), onSelect: () => void sel.selectAll() },
      { label: t("people.invert"), onSelect: () => void sel.invert() },
    );
    return out;
  };
  const openViewerAt = (id: number) => {
    const index = items.findIndex((i) => i.id === id);
    if (index >= 0) setViewer({ faces: items, index });
  };
  const onTileKey = (id: number) => (e: KeyboardEvent) => {
    if (e.key !== "Enter" || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    e.preventDefault();
    openViewerAt(id);
  };
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

  // Valore: id del gruppo, oppure "i:<uid>" per un'identità senza volti qui.
  type MoveOption = {
    value: string;
    label: string;
    name: string;
    correction: (faces: number[]) => Correction;
  };
  const moveOptions = useMemo(() => {
    const seen = new Set<number>();
    const out: MoveOption[] = [];
    for (const g of moveTargets.data?.people ?? []) {
      if (g.id === groupId || seen.has(g.id)) continue;
      seen.add(g.id);
      out.push({
        value: String(g.id),
        label: labelOf(g),
        name: labelOf(g),
        correction: (faces) => ({ action: "move", faces, to: g.id }),
      });
    }
    for (const i of moveTargets.data?.identities ?? []) {
      out.push({
        value: `i:${i.uid}`,
        label: t("people.reassignNoPhotos", { name: i.name }),
        name: i.name,
        correction: (faces) => ({ action: "assign_identity", identity: i.uid, faces }),
      });
    }
    return out.sort((a, b) => a.label.localeCompare(b.label));
  }, [moveTargets.data, groupId, t]); // eslint-disable-line react-hooks/exhaustive-deps

  const moveTarget = moveOptions.find((o) => o.value === moveTo) ?? null;

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
        {busy ? (
          <span role="status" className="people-progress">
            <Spinner size={14} />
            {progress
              ? t("people.progress", { done: progress.done, total: progress.total })
              : t("people.working")}
          </span>
        ) : (
          // Sempre presente: il cambio di conteggio (Ctrl+A, Inverti, Esc) viene annunciato.
          <span aria-live="polite" aria-atomic="true" className="muted">
            {selectedCount > 0 ? t("people.selected", { count: selectedCount }) : null}
          </span>
        )}
        {/* "Riassegna a…": la scelta da sola non sposta nulla (su Windows le frecce cambiano
            una select chiusa); i volti si spostano solo con il pulsante "Riassegna". */}
        <select
          ref={moveSelectRef}
          className="field people-move-select"
          aria-label={t("people.move")}
          value={moveTo}
          disabled={busy || selectedCount === 0}
          onChange={(e) => setMoveTo(e.target.value)}
        >
          <option value="">{t("people.move")}</option>
          {moveOptions.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        <Button
          disabled={busy}
          disabledReason={
            selectedCount === 0
              ? t("people.noSelection")
              : !moveTarget
                ? t("people.moveDisabled")
                : undefined
          }
          onClick={() =>
            moveTarget &&
            correct(
              (ids) => [moveTarget.correction(ids)],
              (n) => t("people.movedDone", { count: n, name: moveTarget.name }),
            )
          }
        >
          {t("people.moveApply")}
        </Button>
        {groupId != null && (
          <>
            <Button disabled={busy || selectedCount === 0} onClick={() => reject(groupId)}>
              {withCount(t("people.reject"), selectedCount)}
            </Button>
            <Button
              disabled={busy || selectedCount === 0}
              onClick={() =>
                correct(
                  (ids) => [{ action: "separate", person: groupId, faces: ids }],
                  (n) => t("people.separatedDone", { count: n }),
                )
              }
            >
              {t("people.newGroup")}
            </Button>
          </>
        )}
        <Button
          variant="danger"
          disabled={busy || selectedCount === 0}
          onClick={() => setRemoveOpen(true)}
        >
          {withCount(t("people.remove"), selectedCount)}
        </Button>
        {discardedOnly && (
          <Button
            disabled={busy || selectedCount === 0}
            onClick={() =>
              correct(
                (ids) => [{ action: "recover", faces: ids }],
                (n) => t("people.recoveredDone", { count: n }),
              )
            }
          >
            {t("people.recover")}
          </Button>
        )}
        <Button disabled={busy || scope == null} onClick={() => void sel.selectAll()}>
          {t("people.selectAll")}
        </Button>
        <Button disabled={busy || scope == null} onClick={() => void sel.invert()}>
          {t("people.invert")}
        </Button>
        <span aria-hidden className="people-toolbar-sep" />
        <label className="muted" htmlFor="people-face-sort">
          {t("people.faceSortLabel")}
        </label>
        <select
          id="people-face-sort"
          className="field"
          value={sort}
          onChange={(e) => setSort(e.target.value as FaceSort)}
        >
          {FACE_SORTS.map((o) => (
            <option key={o} value={o}>
              {t(
                o === "quality_desc"
                  ? "people.faceSortQuality"
                  : o === "captured_asc"
                    ? "people.faceSortDateAsc"
                    : "people.faceSortDateDesc",
              )}
            </option>
          ))}
        </select>
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

      {photoActions.missing.length > 0 && (
        <div className="people-alert">
          <InlineAlert
            tone="warn"
            title={t("faceMenu.missingTitle")}
            onDismiss={photoActions.clearMissing}
          >
            {t("faceMenu.missing", {
              count: photoActions.missing.length,
              paths: photoActions.missing.join(", "),
            })}
          </InlineAlert>
        </div>
      )}
      {photoActions.error && (
        <div className="people-alert">
          <InlineAlert tone="danger" title={t("people.failed")} onDismiss={photoActions.clearError}>
            {photoActions.error}
          </InlineAlert>
        </div>
      )}

      {viewer && (
        <FaceViewer
          faces={viewer.faces}
          index={viewer.index}
          onIndex={(index) => setViewer({ ...viewer, index })}
          onClose={() => setViewer(null)}
        />
      )}

      <ConfirmDialog
        open={removeOpen}
        onOpenChange={setRemoveOpen}
        title={t("people.removeTitle", { count: selectedCount })}
        message={t("people.removeBody")}
        confirmLabel={t("people.removeButton", { count: selectedCount })}
        onConfirm={() =>
          correct((ids) => [{ action: "remove", faces: ids }])
        }
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

      <ContextMenu label={t("faceMenu.label")} items={menuItems()}>
        <div
          ref={scrollRef}
          data-testid="face-grid-scroll"
          onContextMenu={(e) => {
            const tile = (e.target as HTMLElement).closest<HTMLElement>("[data-face-tile]");
            if (!tile) {
              // Fuori da un volto non c'è nulla su cui agire: niente menu.
              e.preventDefault();
              return;
            }
            setMenuFace(Number(tile.dataset.faceId));
          }}
          role="group"
          aria-label={t("people.facesGrid")}
          className="people-grid-scroll"
          onPointerDown={(e) => {
            if (e.button !== 0 || ((e.target as HTMLElement).closest("button") && !e.altKey))
              return;
            const r = e.currentTarget.getBoundingClientRect();
            drag.current = {
              x: e.clientX - r.left,
              y: e.clientY - r.top + e.currentTarget.scrollTop,
              initial: e.ctrlKey || e.metaKey ? selection : null,
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
            const hit = faceRows
              ? rectangleSelectRows(order, b, faceRows, {
                  cell,
                  rowHeight,
                  headHeight: HEAD,
                  gap: GAP,
                  pad: PAD,
                })
              : rectangleSelect(order, b, {
                  cols,
                  cell,
                  rowHeight,
                  gap: GAP,
                  pad: PAD,
                });
            sel.replace(d.initial ? selSet(d.initial, hit, true) : idsSelection<FaceScope>(hit));
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
                  {vitems.map((row) => {
                    const fr = faceRows?.[row.index];
                    if (fr?.kind === "head")
                      return (
                        <div
                          key={row.key}
                          role="heading"
                          aria-level={3}
                          data-testid={`year-${fr.year ?? "none"}`}
                          className="people-year-head"
                          style={{ top: row.start + PAD, height: HEAD }}
                        >
                          {fr.year != null
                            ? t("people.yearHeading", { year: fr.year, count: fr.count })
                            : t("people.noDateHeading", { count: fr.count })}
                        </div>
                      );
                    const from = fr ? fr.from : row.index * cols;
                    const to = fr ? fr.to : from + cols;
                    return (
                      <div
                        key={row.key}
                        className="people-virt-cells"
                        style={{ top: row.start + PAD }}
                      >
                        {items.slice(from, to).map((face) => {
                          const isSel = sel.has(face.id);
                          const lit = highlight?.faceId === face.id;
                          const tip = discardReasonLabel(face.discardReason, face.sizePx, t);
                          return (
                            <button
                              key={face.id}
                              type="button"
                              data-testid={`face-${face.id}`}
                              data-face-tile
                              data-face-id={face.id}
                              onDoubleClick={() => openViewerAt(face.id)}
                              onKeyDown={onTileKey(face.id)}
                              data-highlighted={lit || undefined}
                              aria-pressed={isSel}
                              title={tip}
                              onClick={onFaceClick(face.id)}
                              className="people-face-tile"
                              style={{
                                width: cell,
                                border: `2px solid ${
                                  lit
                                    ? "var(--pc-warn)"
                                    : isSel
                                      ? "var(--pc-accent)"
                                      : "var(--pc-raised)"
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
                    );
                  })}
                </div>
              </>
            )}
          </QueryView>
        </div>
      </ContextMenu>

      <div className="muted people-grid-foot">
        <span>
          {byDate
            ? t("people.faceSortLabel") +
              ": " +
              t(sort === "captured_asc" ? "people.faceSortDateAsc" : "people.faceSortDateDesc")
            : t("people.facesSortHint")}
        </span>
        <span className="mono">
          {fmtInt(items.length)}
          {meta?.total ? ` / ${fmtInt(meta.total)}` : ""}
        </span>
      </div>
    </div>
  );
});
