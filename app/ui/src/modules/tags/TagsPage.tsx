import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type TagInfo, type TagSetView } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { TAG_COLORS, tagColorVar } from "../../theme/tokens";
import { activeSet, errorText, refreshTags, useTagOverview } from "./data";

type Pending =
  | { kind: "merge"; source: string; target: string; fromRename?: boolean }
  | { kind: "delete"; name: string; count: number }
  | { kind: "deleteSet"; set: TagSetView }
  | null;

export function Swatch({ color, size = 14 }: { color: string; size?: number }) {
  return (
    <span
      aria-hidden="true"
      style={{
        display: "inline-block",
        flex: "none",
        width: size,
        height: size,
        borderRadius: 4,
        background: tagColorVar(color),
      }}
    />
  );
}

/** Apre la Libreria filtrata su un tag o su un elenco di file. */
function openInLibrary(filter: { tags?: string[] }, ids?: number[]) {
  const lib = useLibrary.getState();
  lib.resetFilter();
  if (filter.tags) lib.setFilter({ ...emptyFilter(), tags: filter.tags });
  if (ids) lib.setSelection(ids, ids[0] ?? null);
  useLayout.getState().setModule("library");
}

function SetList({
  sets,
  active,
  current,
  onSelect,
  onCreated,
}: {
  sets: TagSetView[];
  active: number | null;
  current: number | null;
  onSelect: (id: number) => void;
  onCreated: (id: number) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const move = (e: KeyboardEvent, i: number) => {
    const next = e.key === "ArrowDown" ? i + 1 : e.key === "ArrowUp" ? i - 1 : -1;
    if (next < 0 || next >= sets.length) return;
    e.preventDefault();
    onSelect(sets[next].id);
    refs.current[next]?.focus();
  };
  const create = async () => {
    setError("");
    try {
      const id = await commands.saveTagSet(null, name.trim(), []);
      setCreating(false);
      setName("");
      await refreshTags(qc);
      onCreated(id);
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <section
      aria-label={t("tags.sets")}
      style={{
        flex: "1 1 240px",
        maxWidth: 280,
        padding: "20px 12px",
        background: "var(--pc-panel-alt)",
        borderRight: "1px solid var(--pc-border)",
        display: "flex",
        flexDirection: "column",
        gap: 8,
        overflow: "auto",
      }}
    >
      <h2 className="section-title" style={{ margin: "0 8px 4px" }}>
        {t("tags.sets")}
      </h2>
      {sets.map((s, i) => {
        const selected = s.id === current;
        const meta = s.isBase
          ? t("tags.metaBase", { count: s.tags.length })
          : s.exclusive
            ? t("tags.metaExclusive", { count: s.tags.length })
            : t("tags.meta", { count: s.tags.length });
        return (
          <button
            key={s.id}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            aria-pressed={selected}
            tabIndex={selected || (current === null && i === 0) ? 0 : -1}
            onClick={() => onSelect(s.id)}
            onKeyDown={(e) => move(e, i)}
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-start",
              gap: 2,
              padding: "10px 12px",
              borderRadius: 6,
              border: 0,
              textAlign: "left",
              cursor: "pointer",
              color: "var(--pc-text)",
              background: selected ? "var(--pc-tree-selected)" : "transparent",
              outline: selected ? "1px solid var(--pc-selection-border)" : undefined,
            }}
          >
            <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <strong style={{ fontWeight: 600 }}>{s.name}</strong>
              {s.isBase && <span className="chip">{t("tags.badgeBase")}</span>}
              {s.id === active && <span className="chip">{t("tags.badgeActive")}</span>}
            </span>
            <span className="muted" style={{ fontSize: 12 }}>
              {meta}
            </span>
          </button>
        );
      })}
      {creating ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
          style={{ display: "flex", flexDirection: "column", gap: 6 }}
        >
          <input
            className="field"
            autoFocus
            aria-label={t("tags.newSetName")}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <div className="feature-actions" style={{ margin: 0 }}>
            <button className="btn btn-primary" type="submit" disabled={!name.trim()}>
              {t("tags.create")}
            </button>
            <button className="btn" type="button" onClick={() => setCreating(false)}>
              {t("tags.cancel")}
            </button>
          </div>
        </form>
      ) : (
        <button
          type="button"
          className="btn"
          style={{ minHeight: 40, borderStyle: "dashed", background: "transparent" }}
          onClick={() => setCreating(true)}
        >
          {t("tags.newSet")}
        </button>
      )}
      {error && <p role="alert">{error}</p>}
      <p className="muted" style={{ margin: 8, fontSize: 12, lineHeight: 1.45 }}>
        {t("tags.baseNote")}
      </p>
    </section>
  );
}

function SetDetail({
  set,
  tags,
  isActive,
  onSelectTag,
  onRenameTag,
  onAskDelete,
  onDuplicated,
}: {
  set: TagSetView;
  tags: TagInfo[];
  isActive: boolean;
  onSelectTag: (name: string) => void;
  onRenameTag: (name: string) => void;
  onAskDelete: () => void;
  onDuplicated: (id: number) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [adding, setAdding] = useState("");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [conflicts, setConflicts] = useState<number[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    setConflicts([]);
    setRenaming(null);
    setError("");
  }, [set.id]);
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await refreshTags(qc);
    } catch (e) {
      setError(errorText(e));
    }
  };
  const count = (name: string) =>
    tags.find((x) => x.name.toLowerCase() === name.toLowerCase())?.count ?? 0;
  const color = (name: string) =>
    tags.find((x) => x.name.toLowerCase() === name.toLowerCase())?.color ?? "gray";
  const freeKey = [1, 2, 3, 4, 5, 6, 7, 8, 9].find((k) => !set.tags.some((x) => x.key === k));
  const grid = "56px 28px minmax(0, 1fr) 100px 150px";
  return (
    <section
      style={{
        flex: "999 1 480px",
        minWidth: 0,
        padding: 24,
        display: "flex",
        flexDirection: "column",
        gap: 16,
      }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
        {renaming === null ? (
          <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{set.name}</h1>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                await commands.saveTagSet(
                  set.id,
                  renaming,
                  set.tags.map((x) => x.name),
                );
                setRenaming(null);
              });
            }}
            style={{ display: "flex", gap: 8 }}
          >
            <input
              className="field"
              autoFocus
              aria-label={t("tags.setName")}
              value={renaming}
              onChange={(e) => setRenaming(e.target.value)}
            />
            <button className="btn btn-primary" type="submit" disabled={!renaming.trim()}>
              {t("tags.save")}
            </button>
            <button className="btn" type="button" onClick={() => setRenaming(null)}>
              {t("tags.cancel")}
            </button>
          </form>
        )}
        <span className="muted">
          {t("tags.setSummary", { tags: set.tags.length, photos: fmtInt(set.tagged) })}
        </span>
        <div style={{ marginLeft: "auto", display: "flex", flexWrap: "wrap", gap: 8 }}>
          {renaming === null && (
            <button className="btn" onClick={() => setRenaming(set.name)}>
              {t("tags.renameSet")}
            </button>
          )}
          <button
            className="btn"
            onClick={() =>
              void run(async () => {
                onDuplicated(await commands.duplicateTagSet(set.id));
              })
            }
          >
            {t("tags.duplicate")}
          </button>
          {!set.isBase && (
            <button className="btn btn-danger" onClick={onAskDelete}>
              {t("tags.deleteSet")}
            </button>
          )}
          <button
            className="btn btn-primary"
            disabled={isActive}
            onClick={() => void run(() => commands.setActiveTagSet(set.id))}
          >
            {isActive ? t("tags.isActive") : t("tags.useActive")}
          </button>
        </div>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
        <label
          className="feature-card"
          style={{ flex: "1 1 260px", flexDirection: "row", alignItems: "flex-start" }}
        >
          <input
            type="checkbox"
            checked={set.exclusive}
            onChange={(e) =>
              void run(async () =>
                setConflicts(await commands.setTagSetExclusive(set.id, e.target.checked)),
              )
            }
          />
          <span>
            <strong>{t("tags.exclusive")}</strong>
            <br />
            <span className="muted" style={{ fontSize: 13 }}>
              {t("tags.exclusiveHelp")}
            </span>
          </span>
        </label>
        <label
          className="feature-card"
          style={{ flex: "1 1 260px", flexDirection: "row", alignItems: "flex-start" }}
        >
          <input
            type="checkbox"
            checked={set.exportKeywords}
            onChange={(e) =>
              void run(() => commands.setTagSetExportKeywords(set.id, e.target.checked))
            }
          />
          <span>
            <strong>{t("tags.exportKeywords")}</strong>
            <br />
            <span className="muted" style={{ fontSize: 13 }}>
              {t("tags.exportKeywordsHelp")}
            </span>
          </span>
        </label>
      </div>
      {conflicts.length > 0 && (
        <div
          role="alert"
          className="feature-actions"
          style={{ margin: 0, color: "var(--pc-warn)" }}
        >
          <span>{t("tags.conflicts", { count: conflicts.length })}</span>
          <button className="btn" onClick={() => openInLibrary({}, conflicts)}>
            {t("tags.openConflicts")}
          </button>
        </div>
      )}
      <div
        role="table"
        aria-label={set.name}
        style={{ border: "1px solid var(--pc-border)", borderRadius: 8, overflow: "hidden" }}
      >
        <div
          role="row"
          style={{
            display: "grid",
            gridTemplateColumns: grid,
            gap: 12,
            alignItems: "center",
            padding: "8px 14px",
            background: "var(--pc-panel)",
            fontSize: 12,
            textTransform: "uppercase",
            letterSpacing: 0.6,
          }}
          className="muted"
        >
          <span role="columnheader">{t("tags.colKey")}</span>
          <span role="columnheader" />
          <span role="columnheader">{t("tags.colTag")}</span>
          <span role="columnheader">{t("tags.colPhotos")}</span>
          <span role="columnheader" />
        </div>
        {set.tags.map((x) => (
          <div
            role="row"
            key={x.key}
            style={{
              display: "grid",
              gridTemplateColumns: grid,
              gap: 12,
              alignItems: "center",
              padding: "6px 14px",
              borderTop: "1px solid var(--pc-raised)",
              minHeight: 44,
            }}
          >
            <span role="cell">
              <kbd
                className="mono"
                style={{
                  width: 28,
                  height: 28,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  borderRadius: 6,
                  background: "var(--pc-raised)",
                  border: "1px solid var(--pc-border)",
                }}
              >
                {x.key}
              </kbd>
            </span>
            <span role="cell">
              <Swatch color={color(x.name)} />
            </span>
            <span role="cell">
              <button
                type="button"
                className="btn btn-ghost"
                style={{ border: 0, fontWeight: 600, padding: 0 }}
                onClick={() => onSelectTag(x.name)}
              >
                {x.name}
              </button>
            </span>
            <span role="cell" className="mono" style={{ fontSize: 13 }}>
              {fmtInt(count(x.name))}
            </span>
            <span role="cell" style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
              <button
                className="btn"
                aria-label={t("tags.renameTag", { name: x.name })}
                onClick={() => onRenameTag(x.name)}
              >
                {t("tags.rename")}
              </button>
              <button
                className="btn icon-btn"
                aria-label={t("tags.removeFromSet", { name: x.name })}
                onClick={() => void run(() => commands.removeTagFromSet(set.id, x.name))}
              >
                ×
              </button>
            </span>
          </div>
        ))}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(async () => {
              await commands.addTagToSet(set.id, adding.trim());
              setAdding("");
            });
          }}
          style={{
            display: "flex",
            gap: 8,
            padding: "10px 14px",
            borderTop: "1px solid var(--pc-raised)",
            background: "var(--pc-panel-alt)",
          }}
        >
          <input
            className="field"
            list="pc-all-tags"
            aria-label={t("tags.addToSetLabel")}
            placeholder={
              freeKey ? t("tags.addToSetPlaceholder", { key: freeKey }) : t("tags.addToSetFull")
            }
            disabled={!freeKey}
            value={adding}
            onChange={(e) => setAdding(e.target.value)}
            style={{ flex: 1, minHeight: 36 }}
          />
          <button className="btn" type="submit" disabled={!freeKey || !adding.trim()}>
            {t("tags.add")}
          </button>
        </form>
      </div>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}

function TagPanel({
  tag,
  sets,
  nameRef,
  onAsk,
  onError,
}: {
  tag: TagInfo | undefined;
  sets: TagSetView[];
  nameRef: RefObject<HTMLInputElement>;
  onAsk: (p: Pending) => void;
  onError: (msg: string) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [name, setName] = useState(tag?.name ?? "");
  const [merging, setMerging] = useState<string | null>(null);
  const [addingTo, setAddingTo] = useState<string | null>(null);
  useEffect(() => {
    setName(tag?.name ?? "");
    setMerging(null);
    setAddingTo(null);
  }, [tag?.name]);
  if (!tag) {
    return <p className="muted">{t("tags.none")}</p>;
  }
  const run = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
      await refreshTags(qc);
    } catch (e) {
      onError(errorText(e));
    }
  };
  const rename = () =>
    run(async () => {
      const out = await commands.renameTag(tag.name, name.trim());
      if (out.outcome === "needsMerge")
        onAsk({ kind: "merge", source: tag.name, target: out.target, fromRename: true });
    });
  return (
    <>
      <div className="muted" style={{ fontSize: 12 }}>
        {t("tags.selected")}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <Swatch color={tag.color} />
        <strong style={{ fontSize: 20 }}>{tag.name}</strong>
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim() && name.trim() !== tag.name) void rename();
        }}
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <label htmlFor="pc-tag-name" style={{ fontWeight: 600 }}>
          {t("tags.name")}
        </label>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            id="pc-tag-name"
            ref={nameRef}
            className="field"
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={{ flex: 1, minHeight: 40 }}
          />
          <button className="btn" type="submit" disabled={!name.trim() || name.trim() === tag.name}>
            {t("tags.rename")}
          </button>
        </div>
      </form>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <span style={{ fontWeight: 600 }}>{t("tags.color")}</span>
        <div role="radiogroup" aria-label={t("tags.color")} style={{ display: "flex", gap: 8 }}>
          {TAG_COLORS.map((c) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={tag.color === c}
              aria-label={t(`tags.colors.${c}`)}
              onClick={() => void run(() => commands.setTagColor(tag.name, c))}
              style={{
                width: 32,
                height: 32,
                borderRadius: 6,
                cursor: "pointer",
                background: tagColorVar(c),
                border: `2px solid ${tag.color === c ? "var(--pc-text)" : "transparent"}`,
              }}
            />
          ))}
        </div>
      </div>
      <div style={{ color: "var(--pc-text-soft)", lineHeight: 1.5 }}>
        {t("tags.usage", { count: tag.count })} ·{" "}
        {tag.sets.length ? t("tags.inSets", { sets: tag.sets.join(", ") }) : t("tags.noSets")}
      </div>
      {tag.count > 0 && (
        <a
          href="#"
          onClick={(e) => {
            e.preventDefault();
            openInLibrary({ tags: [tag.name] });
          }}
        >
          {t("tags.showPhotos", { count: tag.count })}
        </a>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
        {merging === null ? (
          <button className="btn" style={{ minHeight: 40 }} onClick={() => setMerging("")}>
            {t("tags.mergeInto")}
          </button>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              onAsk({ kind: "merge", source: tag.name, target: merging.trim() });
            }}
            style={{ display: "flex", gap: 8 }}
          >
            <input
              className="field"
              autoFocus
              list="pc-all-tags"
              aria-label={t("tags.mergeTarget")}
              value={merging}
              onChange={(e) => setMerging(e.target.value)}
              style={{ flex: 1 }}
            />
            <button
              className="btn"
              type="submit"
              disabled={!merging.trim() || merging.trim().toLowerCase() === tag.name.toLowerCase()}
            >
              {t("tags.merge")}
            </button>
          </form>
        )}
        {addingTo === null ? (
          <button className="btn" style={{ minHeight: 40 }} onClick={() => setAddingTo("")}>
            {t("tags.addToSet")}
          </button>
        ) : (
          <div style={{ display: "flex", gap: 8 }}>
            <select
              className="field"
              autoFocus
              aria-label={t("tags.chooseSet")}
              value={addingTo}
              onChange={(e) => setAddingTo(e.target.value)}
              style={{ flex: 1 }}
            >
              <option value="">{t("tags.chooseSet")}</option>
              {sets
                .filter((s) => !s.tags.some((x) => x.name.toLowerCase() === tag.name.toLowerCase()))
                .map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
            </select>
            <button
              className="btn"
              disabled={!addingTo}
              onClick={() =>
                void run(async () => {
                  await commands.addTagToSet(Number(addingTo), tag.name);
                  setAddingTo(null);
                })
              }
            >
              {t("tags.add")}
            </button>
          </div>
        )}
        <button
          className="btn btn-danger"
          style={{ minHeight: 40 }}
          onClick={() => onAsk({ kind: "delete", name: tag.name, count: tag.count })}
        >
          {t("tags.deleteTag", { count: tag.count })}
        </button>
      </div>
      <p className="muted" style={{ margin: 0, fontSize: 12, lineHeight: 1.45 }}>
        {t("tags.undoNote")}
      </p>
    </>
  );
}

export function TagsPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const overview = useTagOverview();
  const o = overview.data;
  const [setId, setSetId] = useState<number | null>(null);
  const [tagName, setTagName] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [pending, setPending] = useState<Pending>(null);
  const [error, setError] = useState("");
  const nameRef = useRef<HTMLInputElement>(null);
  const current = o?.sets.find((s) => s.id === setId) ?? activeSet(o);
  const tag = o?.tags.find((x) => x.name.toLowerCase() === tagName?.toLowerCase());
  const shown = useMemo(
    () => (o?.tags ?? []).filter((x) => x.name.toLowerCase().includes(filter.trim().toLowerCase())),
    [o, filter],
  );
  const confirm = async () => {
    const p = pending;
    setPending(null);
    setError("");
    try {
      if (p?.kind === "merge") {
        await commands.mergeTag(p.source, p.target);
        setTagName(p.target);
      } else if (p?.kind === "delete") {
        await commands.deleteTag(p.name, true);
        setTagName(null);
      } else if (p?.kind === "deleteSet") {
        await commands.deleteTagSet(p.set.id);
        setSetId(null);
      }
      await refreshTags(qc);
    } catch (e) {
      setError(errorText(e));
    }
  };
  const question =
    pending?.kind === "merge"
      ? t(pending.fromRename ? "tags.renameConflict" : "tags.mergeConfirm", {
          source: pending.source,
          target: pending.target,
        })
      : pending?.kind === "delete"
        ? t("tags.deleteConfirm", { name: pending.name, count: pending.count })
        : pending?.kind === "deleteSet"
          ? t("tags.deleteSetConfirm", { name: pending.set.name })
          : "";
  return (
    <div style={{ flex: 1, display: "flex", flexWrap: "wrap", minWidth: 0, overflow: "auto" }}>
      <datalist id="pc-all-tags">
        {o?.tags.map((x) => (
          <option key={x.name} value={x.name} />
        ))}
      </datalist>
      <SetList
        sets={o?.sets ?? []}
        active={o?.activeSet ?? null}
        current={current?.id ?? null}
        onSelect={setSetId}
        onCreated={setSetId}
      />
      <div style={{ flex: "999 1 480px", minWidth: 0, display: "flex", flexDirection: "column" }}>
        {current && (
          <SetDetail
            set={current}
            tags={o?.tags ?? []}
            isActive={current.id === o?.activeSet}
            onSelectTag={setTagName}
            onRenameTag={(name) => {
              setTagName(name);
              setTimeout(() => nameRef.current?.focus(), 0);
            }}
            onAskDelete={() => setPending({ kind: "deleteSet", set: current })}
            onDuplicated={setSetId}
          />
        )}
        <section
          style={{ padding: "0 24px 24px", display: "flex", flexDirection: "column", gap: 12 }}
        >
          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
            <h2 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>{t("tags.allTags")}</h2>
            <span className="muted">{t("tags.tagCount", { count: o?.tags.length ?? 0 })}</span>
            <input
              type="search"
              className="field"
              aria-label={t("tags.filter")}
              placeholder={t("tags.filterPlaceholder")}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              style={{ marginLeft: "auto", width: 180 }}
            />
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {shown.map((x) => {
              const inSet = current?.tags.some(
                (s) => s.name.toLowerCase() === x.name.toLowerCase(),
              );
              const sel = x.name === tag?.name;
              return (
                <button
                  key={x.name}
                  type="button"
                  className="pill"
                  aria-pressed={sel}
                  onClick={() => setTagName(x.name)}
                  style={{
                    background: sel ? "var(--pc-accent-soft-bg)" : undefined,
                    borderColor: sel
                      ? "var(--pc-accent)"
                      : inSet
                        ? "var(--pc-selection-border)"
                        : undefined,
                  }}
                >
                  <Swatch color={x.color} size={10} />
                  <span>{x.name}</span>
                  <span className="muted mono" style={{ fontSize: 12 }}>
                    {fmtInt(x.count)}
                  </span>
                </button>
              );
            })}
          </div>
        </section>
      </div>
      <aside
        aria-label={t("tags.selected")}
        style={{
          flex: "1 1 300px",
          maxWidth: 340,
          padding: 20,
          background: "var(--pc-panel)",
          borderLeft: "1px solid var(--pc-border)",
          display: "flex",
          flexDirection: "column",
          gap: 14,
        }}
      >
        {pending && (
          <div
            role="alertdialog"
            aria-label={question}
            className="feature-card"
            style={{ alignItems: "stretch", borderColor: "var(--pc-warn)" }}
          >
            <p style={{ margin: 0 }}>{question}</p>
            <div className="feature-actions" style={{ margin: 0 }}>
              <button
                className={pending.kind === "merge" ? "btn btn-primary" : "btn btn-danger"}
                autoFocus
                onClick={() => void confirm()}
              >
                {t("tags.confirm")}
              </button>
              <button className="btn" onClick={() => setPending(null)}>
                {t("tags.cancel")}
              </button>
            </div>
          </div>
        )}
        {error && <p role="alert">{error}</p>}
        <TagPanel
          tag={tag}
          sets={o?.sets ?? []}
          nameRef={nameRef}
          onAsk={setPending}
          onError={setError}
        />
      </aside>
    </div>
  );
}
