import { Coachmark } from "../../components/guide/Coachmark";
import { QueryView } from "../../components/feedback/QueryView";
import { useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type Catalog, type CatalogSummary } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { emptyFilter, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { notify, notifyError } from "../../store/toasts";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  ContextMenu,
  EmptyState,
  FormField,
  InlineAlert,
  PageHeader,
  SegmentedControl,
  type MenuItem,
} from "../../components/ui";
import { Icon } from "../../components/icons";
import { useLabelName } from "../library/labelName";
import { RuleEditor } from "./RuleEditor";
import { newRule, summarize, type RuleNode } from "./rules";
import { catalogLabel, collectDuplicates, startQualityCheck } from "./systemCatalogs";
import { useSelection } from "../../lib/selection/useSelection";
import { handleSelectionKey } from "../../lib/selection/useSelectionKeys";
import { selectForContextMenu } from "../../lib/selection/model";
import { useTypeAhead } from "../../lib/selection/useTypeAhead";
import "./catalogs.css";

const CATALOGS_KEY = ["catalogs"];

/** Apre il catalogo (o le sue foto escluse) in Libreria. */
function openCatalog(id: number, excluded = false) {
  const lib = useLibrary.getState();
  lib.resetFilter();
  lib.setSelection([], null);
  if (excluded) {
    lib.setFilter({ catalogId: null, excludedFrom: id, matchingOnly: true });
    lib.setSort("excluded_at");
  } else lib.setFilter({ catalogId: id });
  useLayout.getState().openModule({ module: "library" });
}

/** Id DOM della scheda: bersaglio di `aria-activedescendant` dell'elenco. */
const catOptionId = (id: number) => `cat-opt-${id}`;

/** Numero di colonne della griglia CSS (1 se non misurabile, es. nei test). */
function gridColumns(el: HTMLElement | null): number {
  const tpl = el ? getComputedStyle(el).gridTemplateColumns : "";
  return Math.max(1, tpl ? tpl.split(" ").filter(Boolean).length : 1);
}

export function CatalogsPage() {
  const { t } = useTranslation();
  const tipAnchor = useRef<HTMLSpanElement>(null);
  const qc = useQueryClient();
  // `undefined`: editor chiuso; `null`: nuovo catalogo.
  const [editing, setEditing] = useState<Catalog | null | undefined>(undefined);
  // Cataloghi da eliminare (uno o più), in attesa di conferma.
  const [deleting, setDeleting] = useState<CatalogSummary[] | null>(null);
  const catalogs = useQuery({
    queryKey: CATALOGS_KEY,
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const list = useMemo(() => catalogs.data ?? [], [catalogs.data]);
  const order = useMemo(() => list.map((s) => s.catalog.id), [list]);

  // Selezione e tastiera (modello di selezione comune, unified-selection).
  const sel = useSelection<null, number>({ filter: null, order });
  const [cursorId, setCursorId] = useState<number | null>(null);
  const cursor = cursorId == null ? -1 : order.indexOf(cursorId);
  const gridRef = useRef<HTMLDivElement>(null);
  const selectedSummaries = () => list.filter((s) => sel.has(s.catalog.id));
  const askDelete = (items: CatalogSummary[]) => {
    if (items.length) setDeleting(items);
  };
  const remove = async (items: CatalogSummary[]) => {
    try {
      if (items.length === 1 && !items[0]!.catalog.system) {
        const c = items[0]!.catalog;
        await commands.deleteCatalog(c.id, true);
        notify({ message: t("catalogs.deleted", { name: c.name }), tone: "ok" });
      } else {
        const out = await commands.deleteCatalogs(
          items.map((s) => s.catalog.id),
          true,
        );
        notify({ message: t("catalogs.deletedMany", { count: out.deleted }), tone: "ok" });
      }
      sel.clear();
      await qc.invalidateQueries({ queryKey: CATALOGS_KEY });
    } catch (e) {
      notifyError(e);
    }
  };
  const typeAhead = useTypeAhead(
    list.map((s) => catalogLabel(s.catalog, t)),
    (i) => {
      const id = order[i];
      if (id == null) return;
      setCursorId(id);
      sel.click(id, { ctrl: false, shift: false });
    },
  );
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === "Enter" && !mod && !e.shiftKey && !e.altKey && cursorId != null) {
      e.preventDefault();
      openCatalog(cursorId);
      return;
    }
    if (e.key === "Delete" && !mod && sel.count > 0) {
      e.preventDefault();
      askDelete(selectedSummaries());
      return;
    }
    if (
      handleSelectionKey(e, {
        layout: "grid",
        columns: gridColumns(gridRef.current),
        pageSize: gridColumns(gridRef.current) * 3,
        count: order.length,
        cursor,
        idAt: (i) => order[i],
        moveTo: (i) => setCursorId(order[i] ?? null),
        select: sel.click,
        ensureAnchor: sel.ensureAnchor,
        selectAll: () => void sel.selectAll(),
        invert: () => void sel.invert(),
        clear: sel.count > 0 ? sel.clear : undefined,
      })
    )
      return;
    typeAhead(e);
  };
  const menuItems = (): MenuItem[] => {
    const items = selectedSummaries();
    const one = items.length === 1 ? items[0]! : null;
    const deletable = items.filter((s) => !s.catalog.system);
    return [
      ...(one
        ? [
            { label: t("catalogs.open"), onSelect: () => openCatalog(one.catalog.id) },
            ...(!one.catalog.system
              ? [{ label: t("catalogs.edit"), onSelect: () => setEditing(one.catalog) }]
              : []),
          ]
        : []),
      {
        label:
          items.length > 1
            ? t("catalogs.deleteMany", { count: items.length })
            : t("catalogs.delete"),
        danger: true,
        disabled: deletable.length === 0,
        onSelect: () => askDelete(items),
      },
    ];
  };
  const skipped = (deleting ?? []).filter((s) => s.catalog.system);
  const toDelete = (deleting ?? []).filter((s) => !s.catalog.system);
  const names = (items: CatalogSummary[]) =>
    items.map((s) => `«${catalogLabel(s.catalog, t)}»`).join(", ");
  return (
    <section className="cat-page">
      <PageHeader
        title={<span ref={tipAnchor}>{t("modules.catalogs")}</span>}
        actions={
          <>
            {sel.count > 1 && (
              <Button
                variant="danger"
                icon={<Icon.Delete size={16} />}
                onClick={() => askDelete(selectedSummaries())}
              >
                {t("catalogs.deleteMany", { count: sel.count })}
              </Button>
            )}
            <Button
              variant="primary"
              icon={<Icon.Plus size={16} />}
              onClick={() => setEditing(null)}
            >
              {t("catalogs.create")}
            </Button>
          </>
        }
      />
      <Coachmark tip="catalogs" anchor={tipAnchor} learnMore="catalogs" />
      {editing !== undefined && (
        <CatalogEditor
          key={editing?.id ?? "new"}
          catalog={editing}
          onClose={() => setEditing(undefined)}
        />
      )}
      <QueryView
        query={catalogs}
        empty={
          editing === undefined && (
            <EmptyState
              icon={<Icon.Catalogs size={32} />}
              title={t("catalogs.emptyTitle")}
              description={t("catalogs.emptyBody")}
              action={
                <Button variant="primary" onClick={() => setEditing(null)}>
                  {t("catalogs.create")}
                </Button>
              }
            />
          )
        }
      >
        {() => (
          <ContextMenu label={t("catalogs.listLabel")} items={menuItems()}>
            <div
              ref={gridRef}
              role="listbox"
              aria-multiselectable="true"
              aria-label={t("catalogs.listLabel")}
              tabIndex={0}
              aria-activedescendant={cursorId != null ? catOptionId(cursorId) : undefined}
              onKeyDown={onKeyDown}
              onClick={(e) => {
                // Clic su uno spazio vuoto (non su una scheda o un pulsante): svuota la selezione.
                if ((e.target as HTMLElement).closest('button, [role="option"]')) return;
                if (e.ctrlKey || e.metaKey || e.shiftKey) return;
                if (sel.count > 0) sel.clear();
              }}
              onContextMenu={(e) => {
                // Fuori da una scheda non c'è nulla su cui agire: niente menu.
                if (!(e.target as HTMLElement).closest('[role="option"]')) e.preventDefault();
              }}
              className="cat-grid"
            >
              {list.map((s) => (
                <CatalogCard
                  key={s.catalog.id}
                  summary={s}
                  selected={sel.has(s.catalog.id)}
                  cursor={s.catalog.id === cursorId}
                  onClick={(e) => {
                    setCursorId(s.catalog.id);
                    sel.click(s.catalog.id, {
                      ctrl: e.ctrlKey || e.metaKey,
                      shift: e.shiftKey,
                    });
                  }}
                  onContextMenu={() => {
                    // Clic destro: su un catalogo non selezionato, selezione solo di quello.
                    const id = s.catalog.id;
                    if (selectForContextMenu(sel.selection, id) !== sel.selection)
                      sel.setIds([id], id);
                    setCursorId(id);
                  }}
                  onEdit={() => setEditing(s.catalog)}
                  onDelete={() => askDelete([s])}
                />
              ))}
            </div>
          </ContextMenu>
        )}
      </QueryView>
      <ConfirmDialog
        open={deleting != null && toDelete.length > 0}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={
          deleting?.length === 1
            ? t("catalogs.deleteTitle", { name: deleting[0]!.catalog.name })
            : t("catalogs.deleteManyTitle", { count: toDelete.length })
        }
        message={
          deleting?.length === 1
            ? t("catalogs.deleteMessage", { count: deleting[0]!.items })
            : [
                t("catalogs.deleteManyMessage", { names: names(toDelete) }),
                skipped.length
                  ? t("catalogs.deleteManySkipped", {
                      count: skipped.length,
                      names: names(skipped),
                    })
                  : "",
              ]
                .filter(Boolean)
                .join(" ")
        }
        files="untouched"
        confirmLabel={
          deleting?.length === 1
            ? t("catalogs.delete")
            : t("catalogs.deleteMany", { count: toDelete.length }).replace("…", "")
        }
        onConfirm={() => deleting && remove(deleting)}
      />
      {deleting != null && toDelete.length === 0 && (
        <InlineAlert tone="warn" onDismiss={() => setDeleting(null)}>
          {t("catalogs.deleteManyOnlySystem")}
        </InlineAlert>
      )}
    </section>
  );
}

function CatalogCard({
  summary: { catalog: c, items, excluded },
  selected,
  cursor,
  onClick,
  onContextMenu,
  onEdit,
  onDelete,
}: {
  summary: CatalogSummary;
  selected: boolean;
  /** La scheda ha il focus della tastiera (distinto dalla selezione). */
  cursor: boolean;
  onClick: (e: MouseEvent) => void;
  onContextMenu: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const smart = c.kind === "smart";
  const label = catalogLabel(c, t);
  return (
    <section className="ui-card cat-card" aria-label={label} data-selected={selected || undefined}>
      {/* La voce selezionabile: solo testo, i comandi stanno accanto (nessun controllo annidato). */}
      <div
        role="option"
        id={catOptionId(c.id)}
        aria-selected={selected}
        aria-label={label}
        data-cursor={cursor || undefined}
        onClick={onClick}
        onDoubleClick={() => openCatalog(c.id)}
        onContextMenu={onContextMenu}
        className="cat-card-body"
      >
        <div className="cat-card-head">
          <h2>{label}</h2>
          <Badge tone={smart ? "info" : "neutral"}>
            {c.system
              ? t("catalogs.system.badge")
              : smart
                ? t("catalogs.smart")
                : t("catalogs.manual")}
          </Badge>
        </div>
        {c.system && <p className="muted">{t(`catalogs.system.help.${c.system}`)}</p>}
        <p className="cat-card-meta">
          <span>{t("catalogs.photos", { count: items, n: fmtInt(items) })}</span>
        </p>
      </div>
      <div className="cat-card-actions">
        {excluded > 0 && (
          <Button variant="ghost" size="sm" onClick={() => openCatalog(c.id, true)}>
            {t("catalogs.excluded", { count: excluded, n: fmtInt(excluded) })}
          </Button>
        )}
        <Button onClick={() => openCatalog(c.id)}>{t("catalogs.open")}</Button>
        {c.system === "duplicates" && (
          <Button variant="ghost" onClick={() => collectDuplicates(qc, t)}>
            {t("catalogs.system.collectDuplicates")}
          </Button>
        )}
        {c.system === "low_quality" && (
          <Button variant="ghost" onClick={() => startQualityCheck(t)}>
            {t("catalogs.system.checkQuality")}
          </Button>
        )}
        {/* I cataloghi di sistema non si modificano né si eliminano. */}
        {!c.system && (
          <>
            <Button variant="ghost" icon={<Icon.Edit size={16} />} onClick={onEdit}>
              {t("catalogs.edit")}
            </Button>
            <Button variant="ghost" icon={<Icon.Delete size={16} />} onClick={onDelete}>
              {t("catalogs.delete")}
            </Button>
          </>
        )}
      </div>
    </section>
  );
}

function CatalogEditor({ catalog, onClose }: { catalog: Catalog | null; onClose: () => void }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const names = useLabelName();
  const [name, setName] = useState(catalog?.name ?? "");
  const [smart, setSmart] = useState(catalog?.kind === "smart");
  const [rule, setRule] = useState<RuleNode>(
    (catalog?.rules as RuleNode | null) ?? { op: "and", rules: [newRule()] },
  );
  const [error, setError] = useState("");
  const preview = useQuery({
    queryKey: ["catalog-preview", rule],
    queryFn: () => commands.libraryCount({ ...emptyFilter(), rule }, null).then((n) => n ?? 0),
    enabled: isTauri() && smart,
    retry: false,
  });
  const save = async () => {
    setError("");
    try {
      await commands.saveCatalog(
        catalog?.id ?? null,
        name.trim(),
        smart ? "smart" : "manual",
        smart ? rule : null,
      );
      onClose();
      await qc.invalidateQueries({ queryKey: CATALOGS_KEY });
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  const previewError = preview.error as { detail?: string } | null;
  return (
    <Card className="cat-editor" title={catalog ? t("catalogs.editTitle") : t("catalogs.newTitle")}>
      <FormField label={t("catalogs.name")}>
        <input className="field" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </FormField>
      <div className="cat-kind">
        <SegmentedControl
          label={t("catalogs.kind")}
          value={smart ? "smart" : "manual"}
          onChange={(v) => setSmart(v === "smart")}
          options={[
            { value: "manual", label: t("catalogs.manual") },
            { value: "smart", label: t("catalogs.smart"), icon: <Icon.Smart size={14} /> },
          ]}
        />
        <span className="muted">{smart ? t("catalogs.smartHint") : t("catalogs.manualHint")}</span>
      </div>
      {smart && (
        <>
          <RuleEditor rule={rule} onChange={setRule} />
          <div className="cat-rule-summary">
            <strong role="status">
              {previewError
                ? String(previewError.detail ?? previewError)
                : preview.isPending
                  ? t("feedback.calculating")
                  : t("catalogs.preview", { count: preview.data })}
            </strong>
            <span>{summarize(t, names, rule)}</span>
          </div>
        </>
      )}
      {error && <InlineAlert tone="danger">{error}</InlineAlert>}
      <div className="cat-editor-actions">
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button
          variant="primary"
          disabled={!name.trim() || (smart && preview.isError)}
          onClick={save}
        >
          {t("common.save")}
        </Button>
      </div>
    </Card>
  );
}
