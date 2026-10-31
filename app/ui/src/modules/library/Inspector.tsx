import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { fmtBytes, fmtDateTime, fmtDec, fmtInt } from "../../lib/format";
import { useLibrary } from "../../store/library";
import { thumbUrl } from "./thumbUrl";

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

const fmtCaptured = (s: string | null | undefined) => (s ? fmtDateTime(s.replace(" ", "T")) : null);

export function Inspector() {
  const { t } = useTranslation();
  const focused = useLibrary((s) => s.focused);
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
                {t(`library.facets.groups.${l.group}`, { defaultValue: l.group })}:{" "}
                {t(`labels.${l.value}`, { defaultValue: l.value.replace(/_/g, " ") })}
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
            <span
              key={tag}
              style={{
                padding: "2px 10px",
                borderRadius: 4,
                background: "var(--pc-raised)",
                border: "1px solid var(--pc-border)",
              }}
            >
              {tag}
            </span>
          ))}
        </div>
        {d.catalogs.length > 0 && (
          <span className="muted" style={{ fontSize: 13 }}>
            {t("library.inspector.catalogs")}: {d.catalogs.join(", ")}
          </span>
        )}
      </section>
    </div>
  );
}
