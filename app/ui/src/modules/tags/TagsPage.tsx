import { Coachmark } from "../../components/guide/Coachmark";
import { useBusyAction } from "../../components/feedback/useBusyAction";
import { isTauri } from "../../lib/ipc";
import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type TagInfo, type TagSetView } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { notify, notifyError } from "../../store/toasts";
import { notifyUndoable } from "../../store/undo";
import { TAG_COLORS, tagColorVar } from "../../theme/tokens";
import { activeSet, errorText, refreshTags, useTagOverview } from "./data";
import { ModulePage } from "../../components/shell/ModulePage";
import { Icon } from "../../components/icons";
import { Button, ConfirmDialog, InlineAlert, InlineEdit } from "../../components/ui";
import "./tags.css";

export function Swatch({ color, size = 14 }: { color: string; size?: number }) {
  return (
    <span
      aria-hidden="true"
      className="tags-swatch"
      style={{ width: size, height: size, background: tagColorVar(color) }}
    />
  );
}

/** Apre la Libreria filtrata su un tag o su un elenco di file. */
function openInLibrary(filter: { tags?: string[] }, ids?: number[]) {
  const lib = useLibrary.getState();
  lib.resetFilter();
  if (filter.tags) lib.setFilter({ ...emptyFilter(), tags: filter.tags });
  if (ids) lib.setSelection(ids, ids[0] ?? null);
  useLayout.getState().openModule({ module: "library" });
}

/**
 * Rinomina, unione ed eliminazione di un tag: si eseguono subito e si annullano dallo
 * store `undo` (avviso con "Annulla operazione" e Ctrl+Z), che chiama `undo_bulk`.
 */
async function undoableTagOperation(qc: QueryClient, run: () => Promise<unknown>, message: string) {
  await run();
  await refreshTags(qc);
  notifyUndoable({
    key: "tags-result",
    message,
    run: async () => {
      await commands.undoBulk();
      await refreshTags(qc);
    },
  });
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
  const tipAnchor = useRef<HTMLHeadingElement>(null);
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
  const [create, actionBusy] = useBusyAction(async () => {
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
  });
  return (
    <section aria-label={t("tags.sets")} className="tags-sets">
      <h2 ref={tipAnchor} className="section-title tags-sets-title">
        {t("tags.sets")}
      </h2>
      <Coachmark tip="tags" anchor={tipAnchor} side="right" learnMore="tags" />
      {sets.map((s, i) => {
        const selected = s.id === current;
        const meta = s.isBase
          ? t("tags.metaBase", { count: s.tags.length })
          : s.exclusive
            ? t("tags.metaExclusive", { count: s.tags.length })
            : t("tags.meta", { count: s.tags.length });
        return (
          <BusyButton
            key={s.id}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            className="tags-set"
            aria-pressed={selected}
            tabIndex={selected || (current === null && i === 0) ? 0 : -1}
            onClick={() => onSelect(s.id)}
            onKeyDown={(e) => move(e, i)}
          >
            <span className="tags-set-name">
              <strong>{s.name}</strong>
              {s.isBase && <span className="chip">{t("tags.badgeBase")}</span>}
              {s.id === active && <span className="chip">{t("tags.badgeActive")}</span>}
            </span>
            <span className="muted tags-small">{meta}</span>
          </BusyButton>
        );
      })}
      {creating ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
          className="tags-stack"
        >
          <input
            className="field"
            autoFocus
            aria-label={t("tags.newSetName")}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <div className="feature-actions tags-actions">
            <BusyButton
              busy={actionBusy}
              className="btn btn-primary"
              type="submit"
              disabled={!name.trim()}
            >
              {t("tags.create")}
            </BusyButton>
            <BusyButton className="btn" type="button" onClick={() => setCreating(false)}>
              {t("tags.cancel")}
            </BusyButton>
          </div>
        </form>
      ) : (
        <BusyButton type="button" className="btn tags-new-set" onClick={() => setCreating(true)}>
          {t("tags.newSet")}
        </BusyButton>
      )}
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
      <p className="muted tags-note">{t("tags.baseNote")}</p>
    </section>
  );
}

type NameProps = {
  label: string;
  value: string;
  busy: boolean;
  /** Restituisce `true` se l'editor può chiudersi. */
  onSave: (name: string) => Promise<boolean>;
};

/** Nome modificabile sul campo (set o tag): l'unico modo di rinominare. */
function NameEdit(props: NameProps) {
  return (
    <InlineEdit label={props.label} value={props.value}>
      {(close) => <NameForm {...props} close={close} />}
    </InlineEdit>
  );
}

function NameForm({ label, value, busy, onSave, close }: NameProps & { close: () => void }) {
  const { t } = useTranslation();
  const [name, setName] = useState(value);
  const changed = name.trim() !== "" && name.trim() !== value;
  return (
    <form
      className="tags-name-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (!changed) return close();
        void onSave(name.trim()).then((ok) => ok && close());
      }}
    >
      <input
        className="field"
        autoFocus
        aria-label={label}
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <BusyButton busy={busy} className="btn btn-primary" type="submit" disabled={!changed}>
        {t("tags.save")}
      </BusyButton>
      <BusyButton className="btn" type="button" onClick={close}>
        {t("tags.cancel")}
      </BusyButton>
    </form>
  );
}

function SetDetail({
  set,
  tags,
  isActive,
  onSelectTag,
  onAskDelete,
  onDuplicated,
}: {
  set: TagSetView;
  tags: TagInfo[];
  isActive: boolean;
  onSelectTag: (name: string) => void;
  onAskDelete: () => void;
  onDuplicated: (id: number) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [adding, setAdding] = useState("");
  const [conflicts, setConflicts] = useState<number[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    setConflicts([]);
    setError("");
  }, [set.id]);
  const [run, actionBusy] = useBusyAction(async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await refreshTags(qc);
      return true;
    } catch (e) {
      setError(errorText(e));
      return false;
    }
  });
  const count = (name: string) =>
    tags.find((x) => x.name.toLowerCase() === name.toLowerCase())?.count ?? 0;
  const color = (name: string) =>
    tags.find((x) => x.name.toLowerCase() === name.toLowerCase())?.color ?? "gray";
  const freeKey = [1, 2, 3, 4, 5, 6, 7, 8, 9].find((k) => !set.tags.some((x) => x.key === k));
  return (
    <section className="tags-detail">
      <div className="tags-detail-head">
        <h2 className="tags-detail-title">
          <NameEdit
            label={t("tags.setName")}
            value={set.name}
            busy={actionBusy}
            onSave={async (name) =>
              (await run(() =>
                commands.saveTagSet(
                  set.id,
                  name,
                  set.tags.map((x) => x.name),
                ),
              )) === true
            }
          />
        </h2>
        <span className="muted">
          {t("tags.setSummary", { tags: set.tags.length, photos: fmtInt(set.tagged) })}
        </span>
        <div className="tags-detail-actions">
          <BusyButton
            className="btn"
            onClick={async () =>
              await run(async () => {
                onDuplicated(await commands.duplicateTagSet(set.id));
              })
            }
          >
            {t("tags.duplicate")}
          </BusyButton>
          {!set.isBase && (
            <Button variant="danger" onClick={onAskDelete}>
              {t("tags.deleteSet")}
            </Button>
          )}
          <BusyButton
            className="btn btn-primary"
            disabled={isActive}
            onClick={async () => await run(() => commands.setActiveTagSet(set.id))}
          >
            {isActive ? t("tags.isActive") : t("tags.useActive")}
          </BusyButton>
        </div>
      </div>
      <div className="tags-options">
        <label className="feature-card tags-option">
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
            <span className="muted tags-help">{t("tags.exclusiveHelp")}</span>
          </span>
        </label>
        <label className="feature-card tags-option">
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
            <span className="muted tags-help">{t("tags.exportKeywordsHelp")}</span>
          </span>
        </label>
      </div>
      {conflicts.length > 0 && (
        <InlineAlert
          tone="warn"
          action={
            <Button onClick={() => openInLibrary({}, conflicts)}>{t("tags.openConflicts")}</Button>
          }
        >
          {t("tags.conflicts", { count: conflicts.length })}
        </InlineAlert>
      )}
      <div role="table" aria-label={set.name} className="tags-table">
        <div role="row" className="tags-row tags-row-head muted">
          <span role="columnheader">{t("tags.colKey")}</span>
          <span role="columnheader" />
          <span role="columnheader">{t("tags.colTag")}</span>
          <span role="columnheader">{t("tags.colPhotos")}</span>
          <span role="columnheader" />
        </div>
        {set.tags.map((x) => (
          <div role="row" key={x.key} className="tags-row">
            <span role="cell">
              <kbd className="mono tags-key">{x.key}</kbd>
            </span>
            <span role="cell">
              <Swatch color={color(x.name)} />
            </span>
            <span role="cell">
              <BusyButton
                type="button"
                className="btn btn-ghost tags-row-name"
                onClick={() => onSelectTag(x.name)}
              >
                {x.name}
              </BusyButton>
            </span>
            <span role="cell" className="mono tags-small">
              {fmtInt(count(x.name))}
            </span>
            <span role="cell" className="tags-row-actions">
              <BusyButton
                className="btn icon-btn"
                aria-label={t("tags.removeFromSet", { name: x.name })}
                onClick={async () => await run(() => commands.removeTagFromSet(set.id, x.name))}
              >
                <Icon.Close size={14} />
              </BusyButton>
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
          className="tags-add"
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
          />
          <BusyButton
            busy={actionBusy}
            className="btn"
            type="submit"
            disabled={!freeKey || !adding.trim()}
          >
            {t("tags.add")}
          </BusyButton>
        </form>
      </div>
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
    </section>
  );
}

function TagPanel({
  tag,
  sets,
  onSelectTag,
}: {
  tag: TagInfo | undefined;
  sets: TagSetView[];
  onSelectTag: (name: string | null) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [merging, setMerging] = useState<string | null>(null);
  const [addingTo, setAddingTo] = useState<string | null>(null);
  // Rinomina verso un nome esistente: l'unione si propone accanto al nome.
  const [conflict, setConflict] = useState<string | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    setMerging(null);
    setAddingTo(null);
    setConflict(null);
    setError("");
  }, [tag?.name]);
  const [run, actionBusy] = useBusyAction(async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      return true;
    } catch (e) {
      setError(errorText(e));
      return false;
    }
  });
  if (!tag) {
    return <p className="muted">{t("tags.none")}</p>;
  }

  const merge = (target: string) =>
    run(() =>
      undoableTagOperation(
        qc,
        async () => {
          await commands.mergeTag(tag.name, target);
          onSelectTag(target);
        },
        t("tags.merged", { source: tag.name, target }),
      ),
    );
  const rename = (to: string) =>
    run(async () => {
      const out = await commands.renameTag(tag.name, to);
      if (out.outcome === "needsMerge") {
        setConflict(out.target);
        return;
      }
      onSelectTag(to);
      await refreshTags(qc);
      notifyUndoable({
        key: "tags-result",
        message: t("tags.renamed", { from: tag.name, to }),
        run: async () => {
          await commands.undoBulk();
          await refreshTags(qc);
        },
      });
    });
  return (
    <>
      <div className="muted tags-small">{t("tags.selected")}</div>
      <div className="tags-panel-name">
        <Swatch color={tag.color} />
        <strong>
          <NameEdit
            label={t("tags.name")}
            value={tag.name}
            busy={actionBusy}
            onSave={async (name) => (await rename(name)) === true}
          />
        </strong>
      </div>
      {conflict && (
        <InlineAlert
          tone="warn"
          action={
            <span className="tags-actions">
              <Button
                variant="primary"
                busy={actionBusy}
                onClick={async () => {
                  const target = conflict;
                  setConflict(null);
                  await merge(target);
                }}
              >
                {t("tags.merge")}
              </Button>
              <Button onClick={() => setConflict(null)}>{t("tags.cancel")}</Button>
            </span>
          }
        >
          {t("tags.renameConflict", { source: tag.name, target: conflict })}
        </InlineAlert>
      )}
      <div className="tags-stack">
        <span className="tags-label">{t("tags.color")}</span>
        <div role="radiogroup" aria-label={t("tags.color")} className="tags-colors">
          {TAG_COLORS.map((c) => (
            <BusyButton
              key={c}
              type="button"
              role="radio"
              className="tags-color"
              aria-checked={tag.color === c}
              aria-label={t(`tags.colors.${c}`)}
              onClick={() =>
                run(async () => {
                  await commands.setTagColor(tag.name, c);
                  await refreshTags(qc);
                })
              }
              style={{ background: tagColorVar(c) }}
            />
          ))}
        </div>
      </div>
      <div className="tags-usage">
        {t("tags.usage", { count: tag.count })} ·{" "}
        {tag.sets.length ? t("tags.inSets", { sets: tag.sets.join(", ") }) : t("tags.noSets")}
      </div>
      {tag.count > 0 && (
        <a
          href="#"
          onClick={(e) => {
            e.preventDefault();
            return openInLibrary({ tags: [tag.name] });
          }}
        >
          {t("tags.showPhotos", { count: tag.count })}
        </a>
      )}
      <div className="tags-stack tags-panel-actions">
        {merging === null ? (
          <BusyButton className="btn tags-tall" onClick={() => setMerging("")}>
            {t("tags.mergeInto")}
          </BusyButton>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void merge(merging.trim()).then((ok) => ok && setMerging(null));
            }}
            className="tags-inline-form"
          >
            <input
              className="field"
              autoFocus
              list="pc-all-tags"
              aria-label={t("tags.mergeTarget")}
              value={merging}
              onChange={(e) => setMerging(e.target.value)}
            />
            <BusyButton
              busy={actionBusy}
              className="btn"
              type="submit"
              disabled={!merging.trim() || merging.trim().toLowerCase() === tag.name.toLowerCase()}
            >
              {t("tags.merge")}
            </BusyButton>
            <BusyButton className="btn" type="button" onClick={() => setMerging(null)}>
              {t("tags.cancel")}
            </BusyButton>
          </form>
        )}
        {addingTo === null ? (
          <BusyButton className="btn tags-tall" onClick={() => setAddingTo("")}>
            {t("tags.addToSet")}
          </BusyButton>
        ) : (
          <div className="tags-inline-form">
            <select
              className="field"
              autoFocus
              aria-label={t("tags.chooseSet")}
              value={addingTo}
              onChange={(e) => setAddingTo(e.target.value)}
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
            <BusyButton
              className="btn"
              disabled={!addingTo}
              onClick={async () =>
                await run(async () => {
                  await commands.addTagToSet(Number(addingTo), tag.name);
                  await refreshTags(qc);
                  setAddingTo(null);
                })
              }
            >
              {t("tags.add")}
            </BusyButton>
          </div>
        )}
        <Button
          variant="danger"
          className="tags-tall"
          busy={actionBusy}
          onClick={() =>
            run(() =>
              undoableTagOperation(
                qc,
                async () => {
                  await commands.deleteTag(tag.name, true);
                  onSelectTag(null);
                },
                t("tags.deleted", { name: tag.name }),
              ),
            )
          }
        >
          {t("tags.deleteTag", { count: tag.count })}
        </Button>
      </div>
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
      <p className="muted tags-note">{t("tags.undoNote")}</p>
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
  // Eliminazione di un set: non ha annullamento lato backend, quindi si conferma.
  const [deleting, setDeleting] = useState<TagSetView | null>(null);
  const current = o?.sets.find((s) => s.id === setId) ?? activeSet(o);
  const tag = o?.tags.find((x) => x.name.toLowerCase() === tagName?.toLowerCase());
  const shown = useMemo(
    () => (o?.tags ?? []).filter((x) => x.name.toLowerCase().includes(filter.trim().toLowerCase())),
    [o, filter],
  );
  const deleteSet = async (set: TagSetView) => {
    try {
      await commands.deleteTagSet(set.id);
      setSetId(null);
      await refreshTags(qc);
      notify({ key: "tags-result", message: t("tags.setDeleted", { name: set.name }), tone: "ok" });
    } catch (e) {
      notifyError(e);
    }
  };
  return (
    <ModulePage title={t("modules.tags")}>
      <QueryView query={overview} enabled={isTauri()} isEmpty={() => false}>
        {() => (
          <div className="tags-page">
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
            <div className="tags-main">
              {current && (
                <SetDetail
                  key={current.id}
                  set={current}
                  tags={o?.tags ?? []}
                  isActive={current.id === o?.activeSet}
                  onSelectTag={setTagName}
                  onAskDelete={() => setDeleting(current)}
                  onDuplicated={setSetId}
                />
              )}
              <section className="tags-all">
                <div className="tags-all-head">
                  <h2>{t("tags.allTags")}</h2>
                  <span className="muted">
                    {t("tags.tagCount", { count: o?.tags.length ?? 0 })}
                  </span>
                  <input
                    type="search"
                    className="field"
                    aria-label={t("tags.filter")}
                    placeholder={t("tags.filterPlaceholder")}
                    value={filter}
                    onChange={(e) => setFilter(e.target.value)}
                  />
                </div>
                <div className="tags-pills">
                  {shown.map((x) => {
                    const inSet = current?.tags.some(
                      (s) => s.name.toLowerCase() === x.name.toLowerCase(),
                    );
                    const sel = x.name === tag?.name;
                    return (
                      <BusyButton
                        key={x.name}
                        type="button"
                        className="pill"
                        aria-pressed={sel}
                        data-in-set={inSet || undefined}
                        onClick={() => setTagName(x.name)}
                      >
                        <Swatch color={x.color} size={10} />
                        <span>{x.name}</span>
                        <span className="muted mono tags-small">{fmtInt(x.count)}</span>
                      </BusyButton>
                    );
                  })}
                </div>
              </section>
            </div>
            <aside aria-label={t("tags.selected")} className="tags-panel">
              <TagPanel tag={tag} sets={o?.sets ?? []} onSelectTag={setTagName} />
            </aside>
          </div>
        )}
      </QueryView>
      <ConfirmDialog
        open={deleting != null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t("tags.deleteSetTitle", { name: deleting?.name ?? "" })}
        message={t("tags.deleteSetMessage")}
        confirmLabel={t("tags.deleteSet")}
        onConfirm={() => deleting && deleteSet(deleting)}
      />
    </ModulePage>
  );
}
