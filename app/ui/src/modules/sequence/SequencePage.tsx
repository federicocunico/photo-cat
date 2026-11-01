import { useEffect, useMemo, useRef, useState, type DragEvent, type MouseEvent } from "react";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type SequenceItem,
  type SequencePage as Page,
  type SequenceScope,
  type SessionView,
  type SourceGroupView,
  type UndatedItem,
} from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { clickSelect, useLibrary } from "../../store/library";
import { offsetText, parseShift, shiftText, timeOf, useSequence } from "../../store/sequence";
import { deviceColorVar } from "../../theme/tokens";
import { thumbUrl } from "../library/thumbUrl";
import { activeSet, colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";
import { isTyping, useTagHotkeys } from "../tags/useTagHotkeys";
import { SequenceExport } from "./SequenceExport";

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

/** Barra con ambito, soglia delle sessioni e tasti del set attivo. */
function Toolbar({ gap }: { gap: number }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const scope = useSequence((s) => s.scope);
  const setScope = useSequence((s) => s.setScope);
  const libSelection = useLibrary((s) => s.selected);
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
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: 14,
        padding: "10px 16px",
        borderBottom: "1px solid var(--pc-border)",
        background: "var(--pc-toolbar)",
      }}
    >
      <label className="muted" style={{ display: "flex", alignItems: "center", gap: 6 }}>
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
        <strong style={{ color: "var(--pc-text)" }}>{t("sequence.realInstant")}</strong>
      </span>
      <label className="muted" style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {t("sequence.gap")}
        <input
          className="field"
          type="number"
          min={1}
          aria-label={t("sequence.gap")}
          defaultValue={gap}
          key={gap}
          style={{ width: 72 }}
          onBlur={(e) => {
            const n = Number(e.target.value);
            if (n > 0 && n !== gap) void commands.setSessionGap(n).then(() => refreshSequence(qc));
          }}
        />
      </label>
      {set && (
        <div
          role="group"
          aria-label={t("sequence.hotkeys")}
          style={{
            marginLeft: "auto",
            display: "flex",
            flexWrap: "wrap",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span className="muted">{t("sequence.activeSet", { name: set.name })}</span>
          {set.tags.map((x) => (
            <span key={x.key} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
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
  tagColor,
  onClick,
  onDropUndated,
}: {
  item: SequenceItem;
  selected: boolean;
  color: string;
  tagColor: (n: string) => string;
  onClick: (e: MouseEvent) => void;
  onDropUndated: (media: number) => void;
}) {
  const { t } = useTranslation();
  const url = thumbUrl(item.thumb);
  const [over, setOver] = useState(false);
  return (
    <button
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
      style={{
        width: CELL,
        padding: 0,
        borderRadius: 8,
        overflow: "hidden",
        cursor: "pointer",
        textAlign: "left",
        color: "inherit",
        background: "var(--pc-panel)",
        border: 0,
        outline: selected
          ? "2px solid var(--pc-accent)"
          : over
            ? "2px dashed var(--pc-accent)"
            : "1px solid var(--pc-raised)",
      }}
    >
      <div
        style={{
          position: "relative",
          aspectRatio: "3 / 2",
          background: "var(--pc-thumb-bg)",
        }}
      >
        {url && (
          <img
            src={url}
            alt=""
            loading="lazy"
            style={{ width: "100%", height: "100%", objectFit: "cover" }}
          />
        )}
        {item.tags[0] && (
          <span style={{ position: "absolute", left: 6, bottom: 6 }}>
            <TagChip name={item.tags[0]} color={tagColor(item.tags[0])} compact />
          </span>
        )}
      </div>
      <div aria-hidden="true" style={{ height: 4, background: color }} />
      <div
        className="mono"
        style={{
          display: "flex",
          justifyContent: "space-between",
          gap: 6,
          padding: "4px 8px",
          fontSize: 11,
        }}
      >
        <span title={item.presumed ? t("sequence.presumed") : undefined}>
          {timeOf(item.local)}
          {item.presumed ? "*" : ""}
        </span>
        <span
          className="muted"
          style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
        >
          {item.device ?? t("sequence.otherSources")}
        </span>
      </div>
    </button>
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
  const rows = useMemo(() => {
    const out: Row[] = [];
    let i = 0;
    for (const s of sessions) {
      const members: SequenceItem[] = [];
      while (i < items.length && items[i].session === s.index) members.push(items[i++]);
      if (!members.length && i >= items.length) break;
      out.push({ kind: "header", session: s });
      for (let k = 0; k < members.length; k += cols)
        out.push({ kind: "cards", items: members.slice(k, k + cols) });
    }
    return out;
  }, [sessions, items, cols]);
  const order = useMemo(() => items.map((i) => i.id), [items]);
  const color = (g: number | null) =>
    deviceColorVar(
      Math.max(
        0,
        groups.findIndex((x) => x.id === g),
      ),
    );
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
    <div
      ref={scrollRef}
      data-testid="sequence-scroll"
      style={{ flex: 1, overflow: "auto", position: "relative" }}
    >
      <div style={{ height: virt.getTotalSize() + PAD, position: "relative" }}>
        {vitems.map((v) => {
          const row = rows[v.index];
          return (
            <div
              key={v.key}
              style={{
                position: "absolute",
                top: v.start + PAD / 2,
                left: PAD,
                right: PAD,
                display: "flex",
                gap: GAP,
              }}
            >
              {row.kind === "header" ? (
                <div style={{ display: "flex", alignItems: "center", gap: 12, width: "100%" }}>
                  <h2 style={{ margin: 0, fontSize: 16 }}>
                    {row.session.startLocal.slice(0, 10)} ·{" "}
                    {timeOf(row.session.startLocal).slice(0, 5)}–
                    {timeOf(row.session.endLocal).slice(0, 5)}
                  </h2>
                  <span className="muted">
                    {[row.session.place, t("sequence.photos", { count: row.session.count })]
                      .filter(Boolean)
                      .join(" · ")}
                  </span>
                  <button
                    className="btn"
                    style={{ marginLeft: "auto" }}
                    onClick={() => void selectSession(row.session.index)}
                  >
                    {t("sequence.selectSession")}
                  </button>
                </div>
              ) : (
                row.items.map((it) => (
                  <Card
                    key={it.id}
                    item={it}
                    selected={selected.includes(it.id)}
                    color={color(it.groupId)}
                    tagColor={(n) => colorOf(tags, n)}
                    onDropUndated={(media) => onPlace(media, it.id)}
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
    <section
      aria-label={t("sequence.undated", { count: items.length })}
      style={{
        borderTop: "1px solid var(--pc-border)",
        padding: "10px 16px",
        display: "flex",
        flexDirection: "column",
        gap: 8,
      }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", gap: 12, alignItems: "baseline" }}>
        <strong>{t("sequence.undated", { count: items.length })}</strong>
        <span className="muted" style={{ fontSize: 12 }}>
          {t("sequence.undatedHelp")}
        </span>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
        {items.map((u) => {
          const url = thumbUrl(u.thumb);
          return (
            <div key={u.id} style={{ display: "flex", flexDirection: "column", gap: 4, width: 96 }}>
              <span
                draggable
                data-testid={`undated-${u.id}`}
                title={u.name}
                onDragStart={(e) =>
                  e.dataTransfer.setData("application/x-photocat-undated", String(u.id))
                }
                style={{
                  display: "block",
                  width: 96,
                  height: 64,
                  borderRadius: 6,
                  cursor: "grab",
                  background: url ? `center / cover url("${url}")` : "var(--pc-thumb-bg)",
                }}
              />
              <button
                className="btn"
                style={{ minHeight: 26, fontSize: 12, padding: "0 6px" }}
                onClick={() => {
                  setMenu(u.id);
                  setAfter(String(focused ?? candidates[0]?.id ?? ""));
                }}
              >
                {t("sequence.placeAfter")}
              </button>
            </div>
          );
        })}
      </div>
      {menu !== null && (
        <div
          role="dialog"
          aria-label={t("sequence.placeAfter")}
          className="feature-actions"
          style={{ margin: 0 }}
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
                {c.local.slice(0, 16)} · {c.name}
              </option>
            ))}
          </select>
          <button
            className="btn btn-primary"
            disabled={!after}
            onClick={() => {
              onPlace(menu, Number(after));
              setMenu(null);
            }}
          >
            {t("sequence.place")}
          </button>
          <button className="btn" onClick={() => setMenu(null)}>
            {t("sequence.cancel")}
          </button>
        </div>
      )}
    </section>
  );
}

type Staged = { tz: string | null; offset: number | null; shift: number; shiftText: string };

function SourcesPanel({ groups }: { groups: SourceGroupView[] }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const selected = useSequence((s) => s.selected);
  const setView = useSequence((s) => s.setView);
  const [staged, setStaged] = useState<Record<number, Staged>>({});
  const [error, setError] = useState("");
  const [note, setNote] = useState("");
  const [previews, setPreviews] = useState<Record<number, string>>({});
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
    <aside
      aria-label={t("sequence.sources")}
      style={{
        flex: "0 0 320px",
        padding: 16,
        background: "var(--pc-panel)",
        borderLeft: "1px solid var(--pc-border)",
        display: "flex",
        flexDirection: "column",
        gap: 12,
        overflow: "auto",
      }}
    >
      <h2 style={{ margin: 0, fontSize: 18 }}>{t("sequence.sources")}</h2>
      <span className="muted" style={{ fontSize: 12, lineHeight: 1.45 }}>
        {t("sequence.sourcesHelp")}
      </span>
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
            className="feature-card"
            style={{
              alignItems: "stretch",
              gap: 6,
              borderColor: changed(g) ? "var(--pc-accent)" : undefined,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span
                aria-hidden="true"
                style={{ width: 12, height: 12, borderRadius: 3, background: deviceColorVar(i) }}
              />
              <strong style={{ flex: 1 }}>
                {name}
                {g.declaredOffsetMin != null && (
                  <span className="muted mono" style={{ fontWeight: 400 }}>
                    {" "}
                    {offsetText(g.declaredOffsetMin)}
                  </span>
                )}
              </strong>
              <span className="muted">{t("sequence.photos", { count: g.count })}</span>
            </div>
            <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12 }}>
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
            <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: 12 }}>
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
              <span className="mono" style={{ fontSize: 12 }} data-testid={`preview-${g.id}`}>
                {previews[g.id]}
              </span>
            )}
          </div>
        );
      })}
      <button className="btn" disabled={selected.length !== 2} onClick={() => void align()}>
        {t("sequence.align")}
      </button>
      {selected.length !== 2 && (
        <span className="muted" style={{ fontSize: 12 }}>
          {t("sequence.alignHelp")}
        </span>
      )}
      {note && (
        <p role="status" style={{ margin: 0 }}>
          {note}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      <div className="feature-actions" style={{ margin: 0 }}>
        <button
          className="btn"
          disabled={!dirty.length}
          onClick={() => {
            setStaged({});
            setPreviews({});
          }}
        >
          {t("sequence.discard")}
        </button>
        <button className="btn btn-primary" disabled={!dirty.length} onClick={() => void apply()}>
          {t("sequence.apply", { count: dirty.length })}
        </button>
      </div>
      <button className="btn" onClick={() => setView("export")}>
        {t("sequence.export")}
      </button>
    </aside>
  );
}

function Viewer({ item, onClose }: { item: SequenceItem; onClose: () => void }) {
  const url = thumbUrl(item.thumb);
  return (
    <div
      role="dialog"
      aria-label={item.name}
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 50,
        background: "rgba(0,0,0,0.88)",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
        color: "#E8E8EA",
      }}
    >
      {url && <img src={url} alt="" style={{ maxWidth: "92vw", maxHeight: "84vh" }} />}
      <span className="mono">
        {item.local} · {item.device ?? ""} · {item.name}
      </span>
    </div>
  );
}

export function SequencePage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const scope = useSequence((s) => s.scope);
  const view = useSequence((s) => s.view);
  const setStats = useSequence((s) => s.setStats);
  const [error, setError] = useState("");
  const [viewer, setViewer] = useState<SequenceItem | null>(null);
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
  const o = overview.data;
  useEffect(() => {
    if (o)
      setStats({
        total: o.total,
        devices: o.groups.filter((g) => g.device).length,
        sessions: o.sessions.length,
      });
  }, [o, setStats]);
  useTagHotkeys(() => useSequence.getState().selected, view === "sequence");
  useEffect(() => {
    if (view !== "sequence") return;
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return;
      const { focused } = useSequence.getState();
      const item = items.find((i) => i.id === focused);
      if (e.key === "Escape") setViewer(null);
      if (!item) return;
      if (e.key === "a" || e.key === "A") {
        e.preventDefault();
        void commands
          .sequenceSessionIds(scope, item.session)
          .then((ids) => useSequence.getState().setSelection(ids, item.id));
      } else if (e.key === " ") {
        e.preventDefault();
        setViewer((v) => (v ? null : item));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, scope, view]);
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
    <div style={{ flex: 1, display: "flex", minWidth: 0, minHeight: 0 }}>
      <section style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
        <Toolbar gap={o?.gapMin ?? 120} />
        {o?.suggestions.map((s) => {
          const g = o.groups.find((x) => x.id === s.group);
          return (
            <div
              key={s.group}
              role="alert"
              style={{
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                gap: 10,
                padding: "10px 16px",
                background: "var(--pc-warn-bg)",
                color: "var(--pc-warn)",
                borderBottom: "1px solid var(--pc-border)",
              }}
            >
              <span style={{ flex: 1, color: "var(--pc-text)" }}>
                {t("sequence.suggestion", {
                  device: groupName(g, t),
                  count: s.count,
                  current: offsetText(s.currentOffsetMin),
                  offset: offsetText(s.offsetMin),
                })}
              </span>
              <button
                className="btn btn-primary"
                onClick={() =>
                  void commands
                    .setGroupDisplay(s.group, s.tz, s.tz ? null : s.offsetMin)
                    .then(() => refreshSequence(qc))
                    .catch((e) => setError(errorText(e)))
                }
              >
                {t("sequence.applySuggestion", { offset: offsetText(s.offsetMin) })}
              </button>
              <button
                className="btn"
                onClick={() =>
                  void commands
                    .ignoreTzSuggestion(s.group, s.offsetMin)
                    .then(() => qc.invalidateQueries({ queryKey: ["sequence"] }))
                }
              >
                {t("sequence.ignore")}
              </button>
            </div>
          );
        })}
        {error && (
          <p role="alert" style={{ margin: "8px 16px" }}>
            {error}
          </p>
        )}
        {o && !o.total && !o.undated.length ? (
          <p className="muted" style={{ padding: 32 }}>
            {t("sequence.empty")}
          </p>
        ) : (
          <SessionsGrid
            sessions={o?.sessions ?? []}
            items={items}
            groups={o?.groups ?? []}
            hasMore={!!pages.hasNextPage && !pages.isFetchingNextPage}
            loadMore={() => void pages.fetchNextPage()}
            onPlace={(m, a) => void place(m, a)}
          />
        )}
        <Undated items={o?.undated ?? []} candidates={items} onPlace={(m, a) => void place(m, a)} />
      </section>
      <SourcesPanel groups={o?.groups ?? []} />
      {viewer && <Viewer item={viewer} onClose={() => setViewer(null)} />}
    </div>
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
      <span>{t("sequence.keysHint")}</span>
    </>
  );
}
