import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type BulkAction, type LabelView } from "../../lib/ipc";
import { fmtBytes, fmtDateTime, fmtDec, fmtInt } from "../../lib/format";
import { useLibrary } from "../../store/library";
import { useLive } from "../../store/live";
import { thumbUrl } from "./thumbUrl";
import { colorOf, useTagOverview } from "../tags/data";
import { TagChip } from "../tags/TagChip";
import { PlaceEditor } from "./PlaceEditor";
import { useLabelName } from "./labelName";
import { CategoryPicker } from "./CategoryPicker";

function Grid2({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        gap: "4px 12px",
        fontSize: 13,
      }}
    >
      {children}
    </div>
  );
}

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <>
      <span className="muted">{label}</span>
      <span>{value ?? "—"}</span>
    </>
  );
}

const FINAL = ["completed", "completed_with_errors", "cancelled", "failed"];

/** Correzioni sul file ispezionato: catalogazione, tag e cataloghi (operazioni bulk su un solo id). */
function Edits({ id, labels }: { id: number; labels: LabelView[] }) {
  const { t } = useTranslation();
  const names = useLabelName();
  const qc = useQueryClient();
  const taxonomy = useQuery({
    queryKey: ["taxonomy"],
    queryFn: commands.getTaxonomy,
    enabled: isTauri(),
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const groups = taxonomy.data?.groups ?? [];
  const [group, setGroup] = useState("subject");
  const current = labels.find((l) => l.group === group)?.value ?? "";
  const [value, setValue] = useState(current);
  const [tag, setTag] = useState("");
  const [catalog, setCatalog] = useState("");
  const [job, setJob] = useState<number | null>(null);
  const [error, setError] = useState("");
  const result = useLive((s) => (job === null ? undefined : s.jobs[job]));
  useEffect(() => setValue(current), [current]);
  useEffect(() => {
    if (result && FINAL.includes(result.state)) {
      void qc.invalidateQueries({ queryKey: ["media-detail", id] });
      void qc.invalidateQueries({ queryKey: ["library"] });
      void qc.invalidateQueries({ queryKey: ["catalogs"] });
    }
  }, [result, qc, id]);
  const apply = async (operation: BulkAction) => {
    setError("");
    try {
      setJob(await commands.startBulk([id], operation, false));
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  return (
    <section style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <h3 className="section-title">{t("library.inspector.edit")}</h3>
      <div className="feature-actions" style={{ margin: 0 }}>
        <select
          className="field"
          aria-label={t("library.inspector.group")}
          value={group}
          onChange={(e) => setGroup(e.target.value)}
        >
          {groups.map((g) => (
            <option key={g.id} value={g.id}>
              {names.group(g.id)}
            </option>
          ))}
        </select>
        <CategoryPicker
          ariaLabel={t("library.inspector.value")}
          group={groups.find((g) => g.id === group)}
          value={value}
          onChange={setValue}
          onCreate={async (label) => {
            const r = await commands.addCategory(group, label, [], [id]);
            await qc.invalidateQueries({ queryKey: ["taxonomy"] });
            setValue(r.id);
            if (r.job != null) setJob(r.job);
          }}
        />
        <button
          className="btn"
          disabled={value === current}
          onClick={() => void apply({ action: "set_label", group, value: value || null })}
        >
          {t("bulk.actions.set_label")}
        </button>
      </div>
      <div className="feature-actions" style={{ margin: 0 }}>
        <input
          className="field"
          aria-label={t("library.inspector.tag")}
          value={tag}
          onChange={(e) => setTag(e.target.value)}
        />
        <button
          className="btn"
          disabled={!tag.trim()}
          onClick={() => {
            void apply({ action: "add_tag", name: tag.trim() });
            setTag("");
          }}
        >
          {t("bulk.actions.add_tag")}
        </button>
      </div>
      <div className="feature-actions" style={{ margin: 0 }}>
        <select
          className="field"
          aria-label={t("library.inspector.catalog")}
          value={catalog}
          onChange={(e) => setCatalog(e.target.value)}
        >
          <option value="">{t("catalogs.title")}</option>
          {catalogs.data
            ?.filter((c) => c.catalog.kind === "manual")
            .map(({ catalog: c }) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
        </select>
        <button
          className="btn"
          disabled={!catalog}
          onClick={() => void apply({ action: "add_catalog", catalog: Number(catalog) })}
        >
          {t("bulk.actions.add_catalog")}
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}

const fmtCaptured = (s: string | null | undefined) => (s ? fmtDateTime(s.replace(" ", "T")) : null);

export function Inspector() {
  const { t } = useTranslation();
  const names = useLabelName();
  const focused = useLibrary((s) => s.focused);
  const tags = useTagOverview();
  const detail = useQuery({
    queryKey: ["media-detail", focused],
    queryFn: () => commands.mediaDetail(focused!),
    enabled: isTauri() && focused !== null,
  });
  const d = detail.data;
  if (focused === null || !d) {
    return (
      <p className="muted" style={{ padding: "48px 16px 16px" }}>
        {t("library.inspector.none")}
      </p>
    );
  }
  const url = thumbUrl(d.item.thumb);
  const tech = d.tech;
  const ai = d.labels.filter((l) => l.origin === "auto" || l.origin === "manual");
  return (
    <div
      style={{ padding: "48px 16px 16px", display: "flex", flexDirection: "column", gap: 16 }}
      data-testid="inspector"
    >
      <div
        style={{
          aspectRatio: "3 / 2",
          background: "var(--pc-thumb-bg)",
          borderRadius: 4,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          overflow: "hidden",
        }}
      >
        {url ? (
          <img src={url} alt={d.item.name} style={{ maxWidth: "100%", maxHeight: "100%" }} />
        ) : (
          <span className="muted">{d.item.name}</span>
        )}
      </div>
      <section style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <h3 className="section-title">{t("library.inspector.file")}</h3>
        <div
          className="mono"
          style={{ fontSize: 11, color: "var(--pc-text-soft)", wordBreak: "break-all" }}
        >
          {d.item.path}
        </div>
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
        </Grid2>
        {d.duplicates.length > 0 && (
          <span style={{ color: "var(--pc-warn)", fontSize: 13 }}>
            {t("library.inspector.duplicates", { count: d.duplicates.length })}
          </span>
        )}
      </section>
      <section>
        <h3 className="section-title">{t("library.inspector.datesPlace")}</h3>
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
          <Field label={t("library.inspector.modified")} value={fmtCaptured(d.dates?.modifiedAt)} />
          <Field label={t("library.inspector.place")} value={d.location?.name} />
          {d.location && (
            <Field
              label={t("library.inspector.placeSource")}
              value={t(`library.inspector.placeSources.${d.location.source}`)}
            />
          )}
        </Grid2>
        <PlaceEditor
          id={d.item.id}
          name={d.location?.name ?? null}
          source={d.location?.source ?? null}
        />
      </section>
      <section style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <h3 className="section-title">{t("library.inspector.ai")}</h3>
        {ai.length === 0 && d.peopleCount == null && (
          <span className="muted">{t("library.inspector.noAi")}</span>
        )}
        {ai.map((l) => (
          <div key={`${l.group}/${l.value}`}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
              <span>
                {names.group(l.group)}: {names.label(l.group, l.value)}
              </span>
              <span className="mono muted">{fmtDec(l.score ?? 0, 2)}</span>
            </div>
            <div className="bar" style={{ marginTop: 4 }}>
              <span style={{ width: `${Math.round((l.score ?? 0) * 100)}%` }} />
            </div>
          </div>
        ))}
        {d.peopleCount != null && (
          <span style={{ fontSize: 13 }}>
            {t("library.inspector.peopleCount", { n: d.peopleCount })}
          </span>
        )}
      </section>
      <section style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <h3 className="section-title">{t("library.inspector.peopleTags")}</h3>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {d.people.map((p) => (
            <span key={p} className="chip">
              {p}
            </span>
          ))}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {d.tags.map((tag) => (
            <TagChip key={tag} name={tag} color={colorOf(tags.data, tag)} />
          ))}
        </div>
        {d.catalogs.length > 0 && (
          <span className="muted" style={{ fontSize: 13 }}>
            {t("library.inspector.catalogs")}: {d.catalogs.join(", ")}
          </span>
        )}
      </section>
      <Edits key={d.item.id} id={d.item.id} labels={d.labels} />
    </div>
  );
}
