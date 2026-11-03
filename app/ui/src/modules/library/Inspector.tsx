// Pannello di ispezione (desktop-shell, "Pannello di ispezione"): anteprima e nome;
// data, luogo, persone e tag; cataloghi e simili; catalogazioni AI; dati tecnici.
// I campi correggibili si modificano sul campo; con più foto selezionate mostra i
// valori comuni (o "Valori diversi") e la modifica vale per tutte.
import { QueryView } from "../../components/feedback/QueryView";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button, Disclosure, InlineEdit, SearchPicker } from "../../components/ui";
import {
  commands,
  isTauri,
  type BulkAction,
  type LabelView,
  type MediaDetail,
  type MediaFaceView,
  type Target,
} from "../../lib/ipc";
import { fmtBytes, fmtDateTime, fmtDec, fmtInt } from "../../lib/format";
import { selCount, selTarget, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { thumbUrl } from "./thumbUrl";
import { colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";
import { PlaceEditor } from "./PlaceEditor";
import { SimilarSection } from "./SimilarSection";
import { useLabelName, useTaxonomy } from "./labelName";
import { CategoryPicker } from "./CategoryPicker";
import { OpenInPeople } from "../people/OpenInPeople";
import { useCatalogAction } from "./useCatalogAction";
import { useBulkJob } from "./bulkJob";

/** Oltre questo numero di foto non si caricano i dettagli per i valori comuni. */
const COMMON_LIMIT = 200;

function faceChipLabel(face: MediaFaceView, t: (k: string, o?: Record<string, unknown>) => string) {
  if (face.personName) return face.personName;
  if (face.discarded) return t("library.inspector.discardedFace");
  if (face.ignored) return t("library.inspector.ignoredFace");
  return t("library.inspector.unrecognizedFace");
}

function Grid2({ children }: { children: ReactNode }) {
  return <div className="inspector-grid">{children}</div>;
}

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <>
      <span className="muted">{label}</span>
      <span>{value ?? "—"}</span>
    </>
  );
}

/** Sezione richiudibile; lo stato resta anche cambiando foto e dopo il riavvio. */
function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  const key = `inspector.${id}`;
  const collapsed = useLayout((s) => s.collapsedSections.includes(key));
  const toggle = useLayout((s) => s.toggleSection);
  return (
    <Disclosure title={title} level={3} open={!collapsed} onOpenChange={() => toggle(key)}>
      <div className="inspector-section">{children}</div>
    </Disclosure>
  );
}

const fmtCaptured = (s: string | null | undefined) => (s ? fmtDateTime(s.replace(" ", "T")) : null);

/** Valore comune a tutte le foto, `undefined` se diverso. */
function common<T>(values: T[]): T | undefined {
  return values.every((v) => v === values[0]) ? values[0] : undefined;
}

type Editing = {
  target: Target;
  count: number;
  /** Bulk con esito in parole e "Annulla operazione". */
  apply: (operation: BulkAction, name?: string) => Promise<unknown>;
};

function useEditing(target: Target, count: number): Editing {
  const { t } = useTranslation();
  const bulk = useBulkJob();
  return {
    target,
    count,
    apply: (operation, name = "") =>
      bulk.start(target, operation, t(`selection.done.${operation.action}`, { name, count })),
  };
}

/** Luogo modificabile sul campo. */
function PlaceRow({
  edit,
  name,
  source,
  mixed,
}: {
  edit: Editing;
  name: string | null;
  source: string | null;
  mixed?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <>
      <span className="muted">{t("library.inspector.place")}</span>
      <InlineEdit label={t("library.inspector.place")} value={mixed ? t("common.mixed") : name}>
        {(close) => (
          <PlaceEditor
            target={edit.target}
            count={edit.count}
            name={mixed ? null : name}
            source={mixed ? null : source}
            onDone={close}
          />
        )}
      </InlineEdit>
    </>
  );
}

/** Tag con aggiunta sul campo (selettore con ricerca e creazione). */
function TagsRow({ edit, tags }: { edit: Editing; tags: string[] }) {
  const { t } = useTranslation();
  const overview = useTagOverview();
  const options = (overview.data?.tags ?? [])
    .filter((x) => !tags.includes(x.name))
    .map((x) => ({ value: x.name, label: x.name }));
  return (
    <div className="inspector-chips">
      {tags.map((tag) => (
        <TagChip key={tag} name={tag} color={colorOf(overview.data, tag)} />
      ))}
      <InlineEdit
        label={t("library.inspector.addTag")}
        value={null}
        placeholder={t("library.inspector.addTag")}
      >
        {(close) => (
          <SearchPicker
            label={t("selection.tagSearch")}
            placeholder={t("selection.tagSearch")}
            options={options}
            createLabel={(name) => t("selection.tagCreate", { name })}
            onPick={(name) => {
              close();
              return edit.apply({ action: "add_tag", name }, name);
            }}
          />
        )}
      </InlineEdit>
    </div>
  );
}

/** Aggiunta a un catalogo manuale sul campo. */
function AddToCatalog({ edit }: { edit: Editing }) {
  const { t } = useTranslation();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const options = (catalogs.data ?? [])
    .filter((c) => c.catalog.kind === "manual")
    .map((c) => ({ value: String(c.catalog.id), label: c.catalog.name }));
  return (
    <InlineEdit
      label={t("library.inspector.addCatalog")}
      value={null}
      placeholder={t("library.inspector.addCatalog")}
    >
      {(close) => (
        <SearchPicker
          label={t("selection.catalogSearch")}
          options={options}
          emptyLabel={t("selection.noCatalogs")}
          onPick={(value) => {
            close();
            const name = options.find((o) => o.value === value)?.label ?? "";
            return edit.apply({ action: "add_catalog", catalog: Number(value) }, name);
          }}
        />
      )}
    </InlineEdit>
  );
}

/** Una categoria della tassonomia, modificabile sul campo. */
function CategoryRow({
  edit,
  group,
  value,
  score,
}: {
  edit: Editing;
  group: string;
  /** `undefined`: valori diversi tra le foto selezionate. */
  value: string | null | undefined;
  score?: number | null;
}) {
  const { t } = useTranslation();
  const names = useLabelName();
  const taxonomy = useTaxonomy();
  const qc = useQueryClient();
  const [draft, setDraft] = useState(value ?? "");
  const shown = value === undefined ? t("common.mixed") : value ? names.label(group, value) : null;
  return (
    <div className="inspector-category">
      <div className="inspector-category-head">
        <span className="muted">{names.group(group)}</span>
        <InlineEdit label={names.group(group)} value={shown}>
          {(close) => (
            <>
              <CategoryPicker
                ariaLabel={t("library.inspector.value")}
                group={taxonomy.data?.groups.find((g) => g.id === group)}
                value={draft}
                onChange={setDraft}
                onCreate={async (label) => {
                  // La voce nuova è creata e assegnata subito.
                  const r = await commands.addCategory(group, label, [], edit.target);
                  await qc.invalidateQueries({ queryKey: ["taxonomy"] });
                  setDraft(r.id);
                  close();
                }}
              />
              <div className="inspector-actions">
                <Button size="sm" onClick={close}>
                  {t("common.cancel")}
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  disabled={draft === (value ?? "")}
                  onClick={async () => {
                    await edit.apply({ action: "set_label", group, value: draft || null });
                    close();
                  }}
                >
                  {t("common.apply")}
                </Button>
              </div>
            </>
          )}
        </InlineEdit>
        {score != null && <span className="mono muted">{fmtDec(score, 2)}</span>}
      </div>
      {score != null && (
        <div className="bar" aria-hidden="true">
          <span style={{ width: `${Math.round(score * 100)}%` }} />
        </div>
      )}
    </div>
  );
}

/** Gruppi da mostrare: quelli della tassonomia, più quelli presenti nelle etichette. */
function useGroups(labels: LabelView[]) {
  const taxonomy = useTaxonomy();
  const ids = (taxonomy.data?.groups ?? []).map((g) => g.id);
  for (const l of labels) if (!ids.includes(l.group)) ids.push(l.group);
  return ids;
}

function SingleInspector({ d }: { d: MediaDetail }) {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const catalogAction = useCatalogAction();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const edit = useEditing({ kind: "ids", ids: [d.item.id] }, 1);
  const groups = useGroups(d.labels);
  const url = thumbUrl(d.item.thumb);
  const tech = d.tech;
  return (
    <>
      <div className="inspector-preview">
        {url ? <img src={url} alt={d.item.name} /> : <span className="muted">{d.item.name}</span>}
      </div>
      <h2 className="inspector-name">{d.item.name}</h2>

      <Section id="data" title={t("library.inspector.sections.data")}>
        <Grid2>
          <Field label={t("library.inspector.captured")} value={fmtCaptured(d.dates?.capturedAt)} />
          <Field
            label={t("library.inspector.dateSource")}
            value={
              d.dates?.capturedSource
                ? t(`library.inspector.dateSources.${d.dates.capturedSource}`)
                : null
            }
          />
          <PlaceRow
            edit={edit}
            name={d.location?.name ?? null}
            source={d.location?.source ?? null}
          />
          {d.location && (
            <Field
              label={t("library.inspector.placeSource")}
              value={t(`library.inspector.placeSources.${d.location.source}`)}
            />
          )}
        </Grid2>
        {d.faces.length > 0 && (
          <div className="inspector-chips">
            {d.faces.map((face) => (
              <span key={face.id} className="chip" data-testid={`inspector-face-${face.id}`}>
                {faceChipLabel(face, t)}
                <OpenInPeople
                  target={{ face: face.id }}
                  className="btn btn-ghost facet-icon-btn"
                />
              </span>
            ))}
          </div>
        )}
        <TagsRow edit={edit} tags={d.tags} />
      </Section>

      <Section id="collections" title={t("library.inspector.sections.collections")}>
        {d.catalogs.map((name) => {
          const cat = catalogs.data?.find((c) => c.catalog.name === name);
          return (
            <div key={name} className="inspector-row">
              <span>{name}</span>
              {cat && (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={catalogAction.busy}
                  onClick={() => catalogAction.act([d.item.id], cat.catalog.id, name)}
                >
                  {t("exclusions.exclude")}
                </Button>
              )}
            </div>
          );
        })}
        {filter.excludedFrom != null && (
          <Button
            size="sm"
            disabled={catalogAction.busy}
            onClick={() =>
              void catalogAction.act(
                [d.item.id],
                filter.excludedFrom!,
                catalogs.data?.find((c) => c.catalog.id === filter.excludedFrom)?.catalog.name ??
                  "",
                true,
              )
            }
          >
            {t("exclusions.restore")}
          </Button>
        )}
        <AddToCatalog edit={edit} />
        <SimilarSection id={d.item.id} />
      </Section>

      <Section id="ai" title={t("library.inspector.ai")}>
        {d.labels.length === 0 && d.peopleCount == null && (
          <span className="muted">{t("library.inspector.noAi")}</span>
        )}
        {groups.map((g) => {
          const label = d.labels.find((l) => l.group === g);
          return (
            <CategoryRow
              key={g}
              edit={edit}
              group={g}
              value={label?.value ?? null}
              score={label?.origin === "auto" ? label.score : null}
            />
          );
        })}
        {d.peopleCount != null && (
          <span>{t("library.inspector.peopleCount", { n: d.peopleCount })}</span>
        )}
      </Section>

      <Section id="tech" title={t("library.inspector.sections.tech")}>
        <Grid2>
          <Field label={t("library.inspector.format")} value={tech?.format} />
          <Field
            label={t("library.inspector.size")}
            value={tech?.width ? `${fmtInt(tech.width)} × ${fmtInt(tech.height ?? 0)}` : null}
          />
          <Field
            label={t("library.inspector.resolution")}
            value={
              tech?.dpiX
                ? t("library.inspector.dpi", {
                    x: fmtInt(tech.dpiX),
                    y: fmtInt(tech.dpiY ?? tech.dpiX),
                  })
                : null
            }
          />
          <Field
            label={t("library.inspector.depth")}
            value={tech?.bitDepth ? t("library.inspector.bits", { n: tech.bitDepth }) : null}
          />
          <Field label={t("library.inspector.weight")} value={fmtBytes(d.item.size)} />
          {d.exif?.cameraModel && (
            <Field
              label={t("library.inspector.camera")}
              value={[d.exif.cameraMake, d.exif.cameraModel].filter(Boolean).join(" ")}
            />
          )}
          <Field label={t("library.inspector.modified")} value={fmtCaptured(d.dates?.modifiedAt)} />
        </Grid2>
        <div className="mono inspector-path">{d.item.path}</div>
        {d.duplicates.length > 0 && (
          <span className="inspector-warn">
            {t("library.inspector.duplicates", { count: d.duplicates.length })}
          </span>
        )}
      </Section>
    </>
  );
}

/** Più foto selezionate: valori comuni o "Valori diversi", modifiche per tutte. */
function MultiInspector() {
  const { t } = useTranslation();
  const selection = useLibrary((s) => s.selection);
  const count = selCount(selection);
  const target = selTarget(selection);
  const edit = useEditing(target, count);
  // I dettagli servono solo per i valori comuni di una selezione esplicita non enorme.
  const ids = selection.mode === "ids" && count <= COMMON_LIMIT ? [...selection.ids] : [];
  const details = useQueries({
    queries: ids.map((id) => ({
      queryKey: ["media-detail", id],
      queryFn: () => commands.mediaDetail(id),
      enabled: isTauri(),
    })),
  });
  const loaded = details.map((q) => q.data).filter((d): d is MediaDetail => d != null);
  const known = ids.length > 0 && loaded.length === ids.length;
  const place = known ? common(loaded.map((d) => d.location?.name ?? null)) : undefined;
  const placeSource = known ? common(loaded.map((d) => d.location?.source ?? null)) : undefined;
  const tags = known
    ? loaded.reduce<string[]>(
        (acc, d, i) => (i === 0 ? [...d.tags] : acc.filter((x) => d.tags.includes(x))),
        [],
      )
    : [];
  const groups = useGroups(loaded.flatMap((d) => d.labels));
  return (
    <>
      <h2 className="inspector-name">{t("library.inspector.multiple", { count })}</h2>
      <Section id="data" title={t("library.inspector.sections.data")}>
        <Grid2>
          <PlaceRow
            edit={edit}
            name={place ?? null}
            source={placeSource ?? null}
            mixed={place === undefined}
          />
        </Grid2>
        {tags.length > 0 && <span className="muted">{t("library.inspector.commonTags")}</span>}
        <TagsRow edit={edit} tags={tags} />
      </Section>
      <Section id="collections" title={t("library.inspector.sections.collections")}>
        <AddToCatalog edit={edit} />
      </Section>
      <Section id="ai" title={t("library.inspector.ai")}>
        {groups.map((g) => (
          <CategoryRow
            key={g}
            edit={edit}
            group={g}
            value={
              known
                ? common(loaded.map((d) => d.labels.find((l) => l.group === g)?.value ?? null))
                : undefined
            }
          />
        ))}
      </Section>
    </>
  );
}

export function Inspector() {
  const { t } = useTranslation();
  const focused = useLibrary((s) => s.focused);
  const many = useLibrary((s) => selCount(s.selection) > 1);
  const detail = useQuery({
    queryKey: ["media-detail", focused],
    queryFn: () => commands.mediaDetail(focused!),
    enabled: isTauri() && focused !== null && !many,
  });
  if (many)
    return (
      <div className="inspector" data-testid="inspector">
        <MultiInspector />
      </div>
    );
  if (focused === null)
    return <p className="muted inspector-none">{t("library.inspector.none")}</p>;
  const d = detail.data;
  if (!d)
    return (
      <QueryView query={detail} enabled={isTauri()} isEmpty={() => false}>
        {() => null}
      </QueryView>
    );
  return (
    <div className="inspector" data-testid="inspector">
      <SingleInspector key={d.item.id} d={d} />
    </div>
  );
}
