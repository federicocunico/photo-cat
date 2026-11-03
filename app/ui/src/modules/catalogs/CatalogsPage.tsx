import { Coachmark } from "../../components/guide/Coachmark";
import { QueryView } from "../../components/feedback/QueryView";
import { useRef, useState } from "react";
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
  EmptyState,
  FormField,
  InlineAlert,
  PageHeader,
  SegmentedControl,
} from "../../components/ui";
import { Icon } from "../../components/icons";
import { useLabelName } from "../library/labelName";
import { RuleEditor } from "./RuleEditor";
import { newRule, summarize, type RuleNode } from "./rules";
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

export function CatalogsPage() {
  const { t } = useTranslation();
  const tipAnchor = useRef<HTMLSpanElement>(null);
  const qc = useQueryClient();
  // `undefined`: editor chiuso; `null`: nuovo catalogo.
  const [editing, setEditing] = useState<Catalog | null | undefined>(undefined);
  const [deleting, setDeleting] = useState<CatalogSummary | null>(null);
  const catalogs = useQuery({
    queryKey: CATALOGS_KEY,
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const remove = async (s: CatalogSummary) => {
    try {
      await commands.deleteCatalog(s.catalog.id, true);
      notify({ message: t("catalogs.deleted", { name: s.catalog.name }), tone: "ok" });
      await qc.invalidateQueries({ queryKey: CATALOGS_KEY });
    } catch (e) {
      notifyError(e);
    }
  };
  return (
    <section className="cat-page">
      <PageHeader
        title={<span ref={tipAnchor}>{t("modules.catalogs")}</span>}
        actions={
          <Button variant="primary" icon={<Icon.Plus size={16} />} onClick={() => setEditing(null)}>
            {t("catalogs.create")}
          </Button>
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
        {(list) => (
          <div className="cat-grid">
            {list.map((s) => (
              <CatalogCard
                key={s.catalog.id}
                summary={s}
                onEdit={() => setEditing(s.catalog)}
                onDelete={() => setDeleting(s)}
              />
            ))}
          </div>
        )}
      </QueryView>
      <ConfirmDialog
        open={deleting != null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t("catalogs.deleteTitle", { name: deleting?.catalog.name ?? "" })}
        message={t("catalogs.deleteMessage", { count: deleting?.items ?? 0 })}
        files="untouched"
        confirmLabel={t("catalogs.delete")}
        onConfirm={() => deleting && remove(deleting)}
      />
    </section>
  );
}

function CatalogCard({
  summary: { catalog: c, items, excluded },
  onEdit,
  onDelete,
}: {
  summary: CatalogSummary;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const smart = c.kind === "smart";
  return (
    <Card className="cat-card" aria-label={c.name}>
      <div className="cat-card-head">
        <h2>{c.name}</h2>
        <Badge tone={smart ? "info" : "neutral"}>
          {smart ? t("catalogs.smart") : t("catalogs.manual")}
        </Badge>
      </div>
      <p className="cat-card-meta">
        <span>{t("catalogs.photos", { count: items, n: fmtInt(items) })}</span>
        {excluded > 0 && (
          <>
            <span aria-hidden="true">·</span>
            <Button variant="ghost" size="sm" onClick={() => openCatalog(c.id, true)}>
              {t("catalogs.excluded", { count: excluded, n: fmtInt(excluded) })}
            </Button>
          </>
        )}
      </p>
      <div className="cat-card-actions">
        <Button onClick={() => openCatalog(c.id)}>{t("catalogs.open")}</Button>
        <Button variant="ghost" icon={<Icon.Edit size={16} />} onClick={onEdit}>
          {t("catalogs.edit")}
        </Button>
        <Button variant="ghost" icon={<Icon.Delete size={16} />} onClick={onDelete}>
          {t("catalogs.delete")}
        </Button>
      </div>
    </Card>
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
