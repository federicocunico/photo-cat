import { Coachmark } from "../../components/guide/Coachmark";
import { useEffect, useMemo, useRef, useState, type DragEvent, type MouseEvent } from "react";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type MediaFaceView,
  type SequenceItem,
  type SequencePage as Page,
  type SequenceScope,
  type SessionView,
  type SourceGroupView,
  type UndatedItem,
} from "../../lib/ipc";
import { fmtDate, fmtDateTime, fmtInt } from "../../lib/format";
import { clickSelect, selIds, useLibrary } from "../../store/library";
import { collapseLoaded } from "../library/similarView";
import { Icon } from "../../components/icons";

const EMPTY: number[] = [];
import { offsetText, parseShift, shiftText, timeOf, useSequence } from "../../store/sequence";
import { deviceColorVar } from "../../theme/tokens";
import { thumbUrl } from "../library/thumbUrl";
import { activeSet, colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";
import { hotkeyAction, useTagHotkeys } from "../tags/useTagHotkeys";
import { OpenInPeople } from "../people/OpenInPeople";
import { InlineAlert, Kbd } from "../../components/ui";
import { QueryView } from "../../components/feedback/QueryView";
import { Skeleton } from "../../components/feedback/Skeleton";
import "./sequence.css";
import { ModulePage } from "../../components/shell/ModulePage";
import { Viewer } from "../../components/viewer/Viewer";
import { useBulkJob } from "../library/bulkJob";
import { useCommands } from "../../commands/registry";
import { SequenceExport } from "./SequenceExport";

/** Chip dei volti della foto focalizzata → Apri in Persone. */
function FocusedFaces() {
  const { t } = useTranslation();
  const focused = useSequence((s) => s.focused);
  const detail = useQuery({
    queryKey: ["media-detail", focused],
    queryFn: async () => {
      const d = await commands.mediaDetail(focused!);
      return d ?? { faces: [] as MediaFaceView[] };
    },
    enabled: isTauri() && focused != null,
  });
  const faces = detail.data && "faces" in detail.data ? detail.data.faces : [];
  if (!faces.length) return null;
  return (
    <div role="group" aria-label={t("library.inspector.peopleTags")} className="seq-faces">
      {faces.map((face) => (
        <span key={face.id} className="chip seq-face-chip">
          {face.personName ??
            (face.discarded
              ? t("library.inspector.discardedFace")
              : face.ignored
                ? t("library.inspector.ignoredFace")
                : t("library.inspector.unrecognizedFace"))}
          <OpenInPeople target={{ face: face.id }} className="btn btn-ghost seq-open-people" />
        </span>
      ))}
    </div>
  );
}

const PAGE = 500;
const CELL = 168;
const GAP = 10;
const PAD = 16;
const HEADER = 52;
const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

/** Offset fissi proposti per l'ora locale di un gruppo (minuti). */
const OFFSETS = [
  -720, -660, -600, -570, -540, -480, -420, -360, -300, -240, -210, -180, -120, -60, 0, 60, 120,
  180, 210, 240, 270, 300, 330, 345, 360, 390, 420, 480, 525, 540, 570, 600, 630, 660, 720, 765,
  780, 840,
];

async function refreshSequence(qc: ReturnType<typeof useQueryClient>) {
  await Promise.all([
    qc.invalidateQueries({ queryKey: ["sequence"] }),
    qc.invalidateQueries({ queryKey: ["sequence-items"] }),
    qc.invalidateQueries({ queryKey: ["library"] }),
    qc.invalidateQueries({ queryKey: ["bulk-undo"] }),
  ]);
}

const groupName = (g: SourceGroupView | undefined, t: (k: string) => string) =>
  g?.device ?? t("sequence.otherSources");

/**
 * Ora locale della foto ("AAAA-MM-GG hh:mm:ss") come `Date` nel fuso del computer, così la
 * formattazione mostra la stessa data e ora scritte nella foto.
 */
function localDate(local: string): Date {
  const [y, m, d] = local.slice(0, 10).split("-").map(Number);
  const [hh = 0, mm = 0, ss = 0] = local.slice(11, 19).split(":").map(Number);
  return new Date(y, (m || 1) - 1, d || 1, hh, mm, ss);
}

/**
 * Iniziale di ogni dispositivo (gruppo di origine), mostrata accanto al colore: il colore da
 * solo non basta a distinguere le fotocamere. Se due dispositivi iniziano con la stessa
 * lettera, il secondo ha un numero (F, F2).
 */
function deviceMarks(groups: SourceGroupView[], t: (k: string) => string): Map<number, string> {
  const seen = new Map<string, number>();
  const out = new Map<number, string>();
  for (const g of groups) {
    const letter = (groupName(g, t).match(/[\p{L}\p{N}]/u)?.[0] ?? "?").toUpperCase();
    const n = (seen.get(letter) ?? 0) + 1;
    seen.set(letter, n);
    out.set(g.id, n === 1 ? letter : `${letter}${n}`);
  }
  return out;
}

/** Barra con ambito, soglia delle sessioni e tasti del set attivo. */
function Toolbar({ gap }: { gap: number }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  // Esito visibile della modifica della pausa: salvata, non valida o non riuscita.
  const [gapState, setGapState] = useState<
    | { kind: "saved"; value: number }
    | { kind: "invalid" }
    | { kind: "error"; message: string }
    | null
  >(null);
  const saveGap = (raw: string) => {
    const n = Number(raw);
    if (!raw.trim() || !Number.isFinite(n) || n <= 0) {
      setGapState({ kind: "invalid" });
      return;
    }
    if (n === gap) return;
    void commands
      .setSessionGap(n)
      .then(() => refreshSequence(qc))
      .then(() => setGapState({ kind: "saved", value: n }))
      .catch((e) => setGapState({ kind: "error", message: errorText(e) }));
  };
  const scope = useSequence((s) => s.scope);
  const setScope = useSequence((s) => s.setScope);
  // Una selezione "tutti i risultati" della Libreria non diventa un elenco di id qui.
  const libSelection = useLibrary((s) => selIds(s.selection)) ?? EMPTY;
  const groupSimilar = useLibrary((s) => s.groupSimilar);
  const setGroupSimilar = useLibrary((s) => s.setGroupSimilar);
  const tags = useTagOverview().data;
  const set = activeSet(tags);
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const value =
    scope.scope === "catalog" ? `c${scope.id}` : scope.scope === "selection" ? "sel" : "lib";
  return (
    <div className="seq-toolbar">
      <label className="muted seq-inline-label">
        {t("sequence.scope")}
        <select
          className="field"
          value={value}
          onChange={(e) => {
            const v = e.target.value;
            const next: SequenceScope =
              v === "lib"
                ? { scope: "library" }
                : v === "sel"
                  ? { scope: "selection", ids: libSelection }
                  : { scope: "catalog", id: Number(v.slice(1)) };
            setScope(next);
          }}
        >
          <option value="lib">{t("sequence.scopeLibrary")}</option>
          {catalogs.data?.map(({ catalog: c, items }) => (
            <option key={c.id} value={`c${c.id}`}>
              {t("sequence.scopeCatalog", { name: c.name, count: items })}
            </option>
          ))}
          <option value="sel" disabled={!libSelection.length && scope.scope !== "selection"}>
            {t("sequence.scopeSelection", {
              count: scope.scope === "selection" ? scope.ids.length : libSelection.length,
            })}
          </option>
        </select>
      </label>
      <span className="muted">
        {t("sequence.order")}{" "}
        <strong className="seq-order-strong">{t("sequence.realInstant")}</strong>
      </span>
      <label className="muted seq-inline-label">
        <input
          type="checkbox"
          checked={groupSimilar}
          onChange={(e) => setGroupSimilar(e.target.checked)}
        />
        {t("similar.group")}
      </label>
      <label className="muted seq-inline-label">
        {t("sequence.gap")}
        <input
          className="field seq-gap-input"
          type="number"
          min={1}
          aria-label={t("sequence.gap")}
          aria-invalid={gapState?.kind === "invalid" || undefined}
          defaultValue={gap}
          key={gap}
          onChange={() => setGapState(null)}
          onKeyDown={(e) => {
            if (e.key === "Enter") saveGap(e.currentTarget.value);
          }}
          onBlur={(e) => saveGap(e.target.value)}
        />
      </label>
      {gapState?.kind === "saved" && (
        <span role="status" className="seq-gap-status muted">
          {t("sequence.gapSaved", { count: gapState.value })}
        </span>
      )}
      {gapState?.kind === "invalid" && (
        <span role="alert" className="seq-gap-status ui-text-danger">
          {t("sequence.gapInvalid")}
        </span>
      )}
      {gapState?.kind === "error" && (
        <span role="alert" className="seq-gap-status ui-text-danger">
          {gapState.message}
        </span>
      )}
      {set && (
        <div role="group" aria-label={t("sequence.hotkeys")} className="seq-hotkeys">
          <span className="muted">{t("sequence.activeSet", { name: set.name })}</span>
          {set.tags.map((x) => (
            <span key={x.key} className="seq-hotkey">
              <kbd className="mono">{x.key}</kbd>
              <TagChip name={x.name} color={colorOf(tags, x.name)} />
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function Card({
  item,
  selected,
  color,
  mark,
  tagColor,
  onClick,
  onDropUndated,
  group,
}: {
  item: SequenceItem;
  selected: boolean;
  /** Gruppo di foto simili rappresentato (numero di foto, aperto o chiuso). */
  group?: { count: number; open: boolean; onToggle: () => void };
  color: string;
  /** Iniziale del dispositivo, accanto al colore. */
  mark: string;
  tagColor: (n: string) => string;
  onClick: (e: MouseEvent) => void;
  onDropUndated: (media: number) => void;
}) {
  const { t } = useTranslation();
  const url = thumbUrl(item.thumb);
  const [over, setOver] = useState(false);
  return (
    <div className="seq-card">
      <BusyButton
        type="button"
        data-testid={`seq-${item.id}`}
        aria-pressed={selected}
        aria-label={`${item.name} ${timeOf(item.local)}`}
        onClick={onClick}
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes("application/x-photocat-undated")) {
            e.preventDefault();
            setOver(true);
          }
        }}
        onDragLeave={() => setOver(false)}
        onDrop={(e: DragEvent) => {
          setOver(false);
          const id = Number(e.dataTransfer.getData("application/x-photocat-undated"));
          if (id) onDropUndated(id);
        }}
        className={`seq-thumb-btn${selected ? " seq-thumb-btn--selected" : ""}${over && !selected ? " seq-thumb-btn--over" : ""}`}
      >
        <div className="seq-thumb-frame">
          {url && <img src={url} alt="" loading="lazy" className="seq-thumb-img" />}
          {item.tags[0] && (
            <span className="seq-thumb-tag">
              <TagChip name={item.tags[0]} color={tagColor(item.tags[0])} compact />
            </span>
          )}
        </div>
        <div aria-hidden="true" className="seq-thumb-strip" style={{ background: color }} />
        <div className="mono seq-thumb-meta">
          <span title={item.presumed ? t("sequence.presumed") : undefined}>
            {timeOf(item.local)}
            {item.presumed ? "*" : ""}
          </span>
          <span className="muted seq-thumb-device">
            <span aria-hidden="true" className="seq-device-mark" style={{ borderColor: color }}>
              {mark}
            </span>
            <span className="seq-ellipsis">{item.device ?? t("sequence.otherSources")}</span>
          </span>
        </div>
      </BusyButton>
      {/* Apertura del gruppo: pulsante a parte, sopra la miniatura (non annidato). */}
      {group && (
        <button
          type="button"
          className="seq-group-toggle"
          aria-expanded={group.open}
          aria-label={t(group.open ? "similar.collapse" : "similar.expand", {
            count: group.count,
          })}
          data-testid={`seq-group-${item.id}`}
          onClick={group.onToggle}
        >
          {group.open && <Icon.Minus size={12} />}
          {group.count}
        </button>
      )}
    </div>
  );
}

type Row = { kind: "header"; session: SessionView } | { kind: "cards"; items: SequenceItem[] };

function SessionsGrid({
  sessions,
  items,
  groups,
  hasMore,
  loadMore,
  onPlace,
}: {
  sessions: SessionView[];
  items: SequenceItem[];
  groups: SourceGroupView[];
  hasMore: boolean;
  loadMore: () => void;
  onPlace: (media: number, after: number) => void;
}) {
  const { t } = useTranslation();
  const scope = useSequence((s) => s.scope);
  const { selected, focused, setSelection } = useSequence();
  const tags = useTagOverview().data;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(900);
  useEffect(() => {
    const el = scrollRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([e]) => setWidth(e.contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const cols = Math.max(1, Math.floor((width - PAD * 2 + GAP) / (CELL + GAP)));
  // "Raggruppa foto simili" (stessa preferenza della Libreria): un elemento per gruppo.
  const groupSimilar = useLibrary((s) => s.groupSimilar);
  const [open, setOpen] = useState<ReadonlySet<number>>(new Set());
  const shown = useMemo(
    () =>
      groupSimilar
        ? collapseLoaded(items, open)
        : items.map((item) => ({ item, count: null as number | null })),
    [items, open, groupSimilar],
  );
  const counts = useMemo(() => new Map(shown.map((s) => [s.item.id, s.count])), [shown]);
  const toggle = (g: number) =>
    setOpen((o) => {
      const next = new Set(o);
      if (next.has(g)) next.delete(g);
      else next.add(g);
      return next;
    });
  const rows = useMemo(() => {
    const out: Row[] = [];
    let i = 0;
    const visible = shown.map((s) => s.item);
    for (const s of sessions) {
      const members: SequenceItem[] = [];
      while (i < visible.length && visible[i].session === s.index) members.push(visible[i++]);
      if (!members.length && i >= visible.length) break;
      out.push({ kind: "header", session: s });
      for (let k = 0; k < members.length; k += cols)
        out.push({ kind: "cards", items: members.slice(k, k + cols) });
    }
    return out;
  }, [sessions, shown, cols]);
  const order = useMemo(() => items.map((i) => i.id), [items]);
  const color = (g: number | null) =>
    deviceColorVar(
      Math.max(
        0,
        groups.findIndex((x) => x.id === g),
      ),
    );
  const marks = useMemo(() => deviceMarks(groups, t), [groups, t]);
  const rowHeight = Math.round((CELL * 2) / 3) + 4 + 24 + GAP;
  const virt = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (rows[i]?.kind === "header" ? HEADER : rowHeight),
    overscan: 6,
    initialRect: { width, height: 800 },
  });
  const vitems = virt.getVirtualItems();
  const last = vitems[vitems.length - 1]?.index ?? 0;
  useEffect(() => {
    if (hasMore && last >= rows.length - 4) loadMore();
  }, [last, rows.length, hasMore, loadMore]);
  const selectSession = async (index: number) => {
    const ids = await commands.sequenceSessionIds(scope, index);
    setSelection(ids, ids[0] ?? null);
  };
  return (
    <div ref={scrollRef} data-testid="sequence-scroll" className="seq-scroll">
      <div className="seq-virt" style={{ height: virt.getTotalSize() + PAD }}>
        {vitems.map((v) => {
          const row = rows[v.index];
          return (
            <div key={v.key} className="seq-virt-row" style={{ top: v.start + PAD / 2 }}>
              {row.kind === "header" ? (
                <div className="seq-session-head">
                  <h2 className="seq-session-title">
                    {fmtDate(localDate(row.session.startLocal))} ·{" "}
                    {timeOf(row.session.startLocal).slice(0, 5)}–
                    {timeOf(row.session.endLocal).slice(0, 5)}
                  </h2>
                  <span className="muted">
                    {[row.session.place, t("sequence.photos", { count: row.session.count })]
                      .filter(Boolean)
                      .join(" · ")}
                  </span>
                  <BusyButton
                    className="btn seq-push"
                    onClick={() => selectSession(row.session.index)}
                  >
                    {t("sequence.selectSession")}
                  </BusyButton>
                </div>
              ) : (
                row.items.map((it) => (
                  <Card
                    key={it.id}
                    item={it}
                    selected={selected.includes(it.id)}
                    color={color(it.groupId)}
                    mark={(it.groupId != null && marks.get(it.groupId)) || "?"}
                    tagColor={(n) => colorOf(tags, n)}
                    onDropUndated={(media) => onPlace(media, it.id)}
                    group={
                      counts.get(it.id) != null && it.similarGroup != null
                        ? {
                            count: counts.get(it.id)!,
                            open: open.has(it.similarGroup),
                            onToggle: () => toggle(it.similarGroup!),
                          }
                        : undefined
                    }
                    onClick={(e) =>
                      setSelection(
                        clickSelect(order, selected, focused, it.id, {
                          ctrl: e.ctrlKey || e.metaKey,
                          shift: e.shiftKey,
                        }),
                        it.id,
                      )
                    }
                  />
                ))
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function Undated({
  items,
  candidates,
  onPlace,
}: {
  items: UndatedItem[];
  candidates: SequenceItem[];
  onPlace: (media: number, after: number) => void;
}) {
  const { t } = useTranslation();
  const focused = useSequence((s) => s.focused);
  const [menu, setMenu] = useState<number | null>(null);
  const [after, setAfter] = useState<string>("");
  if (!items.length) return null;
  return (
    <section aria-label={t("sequence.undated", { count: items.length })} className="seq-undated">
      <div className="seq-undated-head">
        <strong>{t("sequence.undated", { count: items.length })}</strong>
        <span className="muted seq-small">{t("sequence.undatedHelp")}</span>
      </div>
      <div className="seq-undated-list">
        {items.map((u) => {
          const url = thumbUrl(u.thumb);
          return (
            <div key={u.id} className="seq-undated-item">
              <span
                draggable
                data-testid={`undated-${u.id}`}
                title={u.name}
                onDragStart={(e) =>
                  e.dataTransfer.setData("application/x-photocat-undated", String(u.id))
                }
                className="seq-undated-thumb"
                style={url ? { background: `center / cover url("${url}")` } : undefined}
              />
              <BusyButton
                className="btn seq-undated-place"
                onClick={() => {
                  setMenu(u.id);
                  setAfter(String(focused ?? candidates[0]?.id ?? ""));
                }}
              >
                {t("sequence.placeAfter")}
              </BusyButton>
            </div>
          );
        })}
      </div>
      {menu !== null && (
        <div
          role="dialog"
          aria-label={t("sequence.placeAfter")}
          className="feature-actions seq-actions-flush"
        >
          <select
            className="field"
            autoFocus
            aria-label={t("sequence.placeAfterTarget")}
            value={after}
            onChange={(e) => setAfter(e.target.value)}
          >
            {candidates.map((c) => (
              <option key={c.id} value={c.id}>
                {fmtDateTime(localDate(c.local))} · {c.name}
              </option>
            ))}
          </select>
          <BusyButton
            className="btn btn-primary"
            disabled={!after}
            onClick={() => {
              onPlace(menu, Number(after));
              setMenu(null);
            }}
          >
            {t("sequence.place")}
          </BusyButton>
          <BusyButton className="btn" onClick={() => setMenu(null)}>
            {t("sequence.cancel")}
          </BusyButton>
        </div>
      )}
    </section>
  );
}

type Staged = { tz: string | null; offset: number | null; shift: number; shiftText: string };

function SourcesPanel({ groups }: { groups: SourceGroupView[] }) {
  const { t } = useTranslation();
  const tipAnchor = useRef<HTMLHeadingElement>(null);
  const qc = useQueryClient();
  const selected = useSequence((s) => s.selected);
  const setView = useSequence((s) => s.setView);
  const [staged, setStaged] = useState<Record<number, Staged>>({});
  const [error, setError] = useState("");
  const [note, setNote] = useState("");
  const [previews, setPreviews] = useState<Record<number, string>>({});
  const marks = useMemo(() => deviceMarks(groups, t), [groups, t]);
  const current = (g: SourceGroupView): Staged =>
    staged[g.id] ?? {
      tz: g.displayTz,
      offset: g.displayOffsetMin,
      shift: g.shiftS,
      shiftText: shiftText(g.shiftS),
    };
  const changed = (g: SourceGroupView) => {
    const s = staged[g.id];
    return !!s && (s.tz !== g.displayTz || s.offset !== g.displayOffsetMin || s.shift !== g.shiftS);
  };
  const dirty = groups.filter(changed);
  const stage = async (g: SourceGroupView, next: Staged) => {
    setStaged((s) => ({ ...s, [g.id]: next }));
    try {
      const p = await commands.previewGroupClock(g.id, next.tz, next.offset, next.shift);
      const first = p[0];
      setPreviews((v) => ({
        ...v,
        [g.id]: first ? `${timeOf(first.before ?? "")} → ${timeOf(first.after ?? "")}` : "",
      }));
    } catch (e) {
      setError(errorText(e));
    }
  };
  const apply = async () => {
    setError("");
    try {
      for (const g of dirty) {
        const s = staged[g.id];
        if (s.tz !== g.displayTz || s.offset !== g.displayOffsetMin)
          await commands.setGroupDisplay(g.id, s.tz, s.tz ? null : s.offset);
        if (s.shift !== g.shiftS) await commands.setGroupShift(g.id, s.shift);
      }
      setStaged({});
      setPreviews({});
      await refreshSequence(qc);
    } catch (e) {
      setError(errorText(e));
    }
  };
  const align = async () => {
    setError("");
    setNote("");
    try {
      const a = await commands.proposeAlignment(selected[0], selected[1]);
      const g = groups.find((x) => x.id === a.group);
      if (!g) return;
      await stage(g, {
        ...current(g),
        shift: a.proposedShiftS,
        shiftText: shiftText(a.proposedShiftS),
      });
      setNote(t("sequence.alignProposal", { name: groupName(g, t), delta: shiftText(a.deltaS) }));
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <aside aria-label={t("sequence.sources")} className="seq-sources">
      <h2 ref={tipAnchor} className="seq-sources-title">
        {t("sequence.sources")}
      </h2>
      <Coachmark tip="sequence" anchor={tipAnchor} side="right" learnMore="sequence" />
      <span className="muted seq-small-lh">{t("sequence.sourcesHelp")}</span>
      {groups.map((g, i) => {
        const c = current(g);
        const name = groupName(g, t);
        const select =
          c.tz != null ? `tz:${c.tz}` : c.offset != null ? `off:${c.offset}` : "declared";
        return (
          <div
            key={g.id}
            role="group"
            aria-label={name}
            className={`feature-card seq-group-card${changed(g) ? " seq-group-card--dirty" : ""}`}
          >
            <div className="seq-group-head">
              <span
                aria-hidden="true"
                className="seq-device-mark"
                style={{ borderColor: deviceColorVar(i) }}
              >
                {marks.get(g.id)}
              </span>
              <strong className="seq-group-name">
                {name}
                {g.declaredOffsetMin != null && (
                  <span className="muted mono seq-declared">
                    {" "}
                    {offsetText(g.declaredOffsetMin)}
                  </span>
                )}
              </strong>
              <span className="muted">{t("sequence.photos", { count: g.count })}</span>
            </div>
            <label className="seq-field-col">
              {t("sequence.localTime")}
              <select
                className="field"
                value={select}
                onChange={(e) => {
                  const v = e.target.value;
                  void stage(g, {
                    ...c,
                    tz: v.startsWith("tz:") ? v.slice(3) : null,
                    offset: v.startsWith("off:") ? Number(v.slice(4)) : null,
                  });
                }}
              >
                <option value="declared">
                  {g.declaredOffsetMin != null
                    ? t("sequence.declared", { offset: offsetText(g.declaredOffsetMin) })
                    : t("sequence.systemOrDeclared")}
                </option>
                {c.tz && <option value={`tz:${c.tz}`}>{c.tz}</option>}
                {OFFSETS.map((o) => (
                  <option key={o} value={`off:${o}`}>
                    UTC{offsetText(o)}
                  </option>
                ))}
              </select>
            </label>
            <label className="seq-field-col">
              {t("sequence.shift")}
              <input
                className="field mono"
                value={c.shiftText}
                aria-invalid={parseShift(c.shiftText) === null}
                onChange={(e) => {
                  const n = parseShift(e.target.value);
                  if (n === null)
                    setStaged((s) => ({ ...s, [g.id]: { ...c, shiftText: e.target.value } }));
                  else void stage(g, { ...c, shift: n, shiftText: e.target.value });
                }}
              />
            </label>
            {changed(g) && previews[g.id] && (
              <span className="mono seq-preview" data-testid={`preview-${g.id}`}>
                {previews[g.id]}
              </span>
            )}
          </div>
        );
      })}
      <BusyButton className="btn" disabled={selected.length !== 2} onClick={() => align()}>
        {t("sequence.align")}
      </BusyButton>
      {selected.length !== 2 && <span className="muted seq-small">{t("sequence.alignHelp")}</span>}
      {note && (
        <p role="status" className="seq-status">
          {note}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <div className="feature-actions seq-actions-flush">
        <BusyButton
          className="btn"
          disabled={!dirty.length}
          onClick={() => {
            setStaged({});
            setPreviews({});
          }}
        >
          {t("sequence.discard")}
        </BusyButton>
        <BusyButton className="btn btn-primary" disabled={!dirty.length} onClick={() => apply()}>
          {t("sequence.apply", { count: dirty.length })}
        </BusyButton>
      </div>
      <BusyButton className="btn" onClick={() => setView("export")}>
        {t("sequence.export")}
      </BusyButton>
    </aside>
  );
}

export function SequencePage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const scope = useSequence((s) => s.scope);
  const view = useSequence((s) => s.view);
  const setStats = useSequence((s) => s.setStats);
  const [error, setError] = useState("");
  // Visualizzatore condiviso: posizione nella sequenza, null se chiuso.
  const [viewerAt, setViewerAt] = useState<number | null>(null);
  const overview = useQuery({
    queryKey: ["sequence", scope],
    queryFn: () => commands.sequenceOverview(scope),
    enabled: isTauri(),
  });
  const pages = useInfiniteQuery({
    queryKey: ["sequence-items", scope],
    queryFn: ({ pageParam }) => commands.sequencePage(scope, pageParam, PAGE),
    initialPageParam: null as Page["next"],
    getNextPageParam: (last) => last.next ?? undefined,
    enabled: isTauri(),
  });
  const items = useMemo(() => pages.data?.pages.flatMap((p) => p.items) ?? [], [pages.data]);
  const tagInfo = useTagOverview().data;
  const bulk = useBulkJob();
  const o = overview.data;
  useEffect(() => {
    if (o)
      setStats({
        total: o.total,
        devices: o.groups.filter((g) => g.device).length,
        sessions: o.sessions.length,
      });
  }, [o, setStats]);
  useTagHotkeys(() => {
    const ids = useSequence.getState().selected;
    return { target: { kind: "ids", ids }, count: ids.length };
  }, view === "sequence");
  // Foto con il focus: A seleziona la sua sessione, Spazio o Invio aprono il visualizzatore.
  const focusedItem = () => items.find((i) => i.id === useSequence.getState().focused);
  useCommands(
    [
      {
        id: "sequence.session",
        label: t("commands.sessionSelect"),
        group: "sequence",
        shortcut: { key: "a" },
        when: () => focusedItem() != null,
        run: () => {
          const item = focusedItem();
          if (!item) return;
          return commands
            .sequenceSessionIds(scope, item.session)
            .then((ids) => useSequence.getState().setSelection(ids, item.id));
        },
      },
      ...[" ", "Enter"].map((key) => ({
        id: `sequence.viewer.${key === " " ? "space" : "enter"}`,
        label: t("commands.openViewer"),
        group: "sequence" as const,
        shortcut: { key },
        palette: key === " ",
        when: () => focusedItem() != null,
        run: () => {
          const item = focusedItem();
          if (item) setViewerAt(items.indexOf(item));
        },
      })),
    ],
    view === "sequence",
  );
  const place = async (media: number, after: number) => {
    setError("");
    try {
      await commands.placeUndated(media, after);
      await refreshSequence(qc);
    } catch (e) {
      setError(errorText(e));
    }
  };
  if (view === "export") return <SequenceExport />;
  return (
    <ModulePage title={t("modules.sequence")}>
      <div className="seq-layout">
        <section className="seq-main">
          <Toolbar gap={o?.gapMin ?? 120} />
          <FocusedFaces />
          {o?.suggestions.map((s) => {
            const g = o.groups.find((x) => x.id === s.group);
            return (
              <div key={s.group} role="alert" className="seq-suggestion">
                <span className="seq-suggestion-text">
                  {t("sequence.suggestion", {
                    device: groupName(g, t),
                    count: s.count,
                    current: offsetText(s.currentOffsetMin),
                    offset: offsetText(s.offsetMin),
                  })}
                </span>
                <BusyButton
                  className="btn btn-primary"
                  onClick={() =>
                    void commands
                      .setGroupDisplay(s.group, s.tz, s.tz ? null : s.offsetMin)
                      .then(() => refreshSequence(qc))
                      .catch((e) => setError(errorText(e)))
                  }
                >
                  {t("sequence.applySuggestion", { offset: offsetText(s.offsetMin) })}
                </BusyButton>
                <BusyButton
                  className="btn"
                  onClick={() =>
                    void commands
                      .ignoreTzSuggestion(s.group, s.offsetMin)
                      .then(() => qc.invalidateQueries({ queryKey: ["sequence"] }))
                  }
                >
                  {t("sequence.ignore")}
                </BusyButton>
              </div>
            );
          })}
          {error && (
            <div className="seq-error-wrap">
              <InlineAlert tone="danger" onDismiss={() => setError("")}>
                {error}
              </InlineAlert>
            </div>
          )}
          {/* Caricamento con QueryView: segnaposto, errore con "Riprova", stato vuoto. */}
          <QueryView
            query={overview}
            enabled={isTauri()}
            skeleton={<Skeleton kind="thumbnail" count={8} />}
            isEmpty={(ov) => !ov.total && !ov.undated.length}
            empty={<p className="muted seq-empty">{t("sequence.empty")}</p>}
          >
            {(ov) => (
              <SessionsGrid
                sessions={ov?.sessions ?? []}
                items={items}
                groups={ov?.groups ?? []}
                hasMore={!!pages.hasNextPage && !pages.isFetchingNextPage}
                loadMore={() => void pages.fetchNextPage()}
                onPlace={(m, a) => void place(m, a)}
              />
            )}
          </QueryView>
          <Undated
            items={o?.undated ?? []}
            candidates={items}
            onPlace={(m, a) => void place(m, a)}
          />
        </section>
        <SourcesPanel groups={o?.groups ?? []} />
        {viewerAt != null && (
          <Viewer
            index={viewerAt}
            total={items.length}
            photo={(() => {
              const it = items[viewerAt];
              return it
                ? {
                    id: it.id,
                    name: it.name,
                    src: thumbUrl(it.thumb),
                    details: (
                      <span className="mono">
                        {[fmtDateTime(localDate(it.local)), it.device].filter(Boolean).join(" · ")}
                      </span>
                    ),
                  }
                : null;
            })()}
            onIndex={(n) => {
              setViewerAt(n);
              // Alla chiusura la sequenza resta sulla stessa foto.
              const it = items[n];
              if (it) useSequence.setState({ focused: it.id });
            }}
            onClose={() => setViewerAt(null)}
            onKey={(key) => {
              const it = items[viewerAt];
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
      </div>
    </ModulePage>
  );
}

/** Conteggi e tasti nella barra di stato della Sequenza. */
export function SequenceStatus() {
  const { t } = useTranslation();
  const stats = useSequence((s) => s.stats);
  const selected = useSequence((s) => s.selected.length);
  return (
    <>
      <span>
        {t("sequence.status", {
          total: fmtInt(stats.total),
          devices: stats.devices,
          sessions: stats.sessions,
        })}
      </span>
      <span>{t("sequence.selected", { count: selected })}</span>
      <span className="status-keys" aria-label={t("sequence.keysLabel")}>
        {(
          [
            [["1–9"], "keys.assign"],
            [["0"], "keys.remove"],
            [["A"], "keys.session"],
            [[t("sequence.keys.space")], "keys.fullscreen"],
          ] as const
        ).map(([keys, label]) => (
          <span key={label}>
            <Kbd keys={[...keys]} /> {t(`sequence.${label}`)}
          </span>
        ))}
      </span>
    </>
  );
}
