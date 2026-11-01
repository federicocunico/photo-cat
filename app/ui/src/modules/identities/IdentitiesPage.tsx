import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type FaceChoice,
  type IdentityInput,
  type IdentityRow,
  type ImportPreview,
} from "../../lib/ipc";
import { fmtDate, fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { FaceCrop } from "../people/PeoplePage";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const KEY = ["identities"] as const;

const initials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join("");

function Avatar({ name, size = 36 }: { name: string; size?: number }) {
  return (
    <span
      aria-hidden="true"
      style={{
        width: size,
        height: size,
        flex: "none",
        borderRadius: "50%",
        background: "var(--pc-thumb-bg)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: size / 3,
        fontWeight: 700,
        color: "var(--pc-text-soft)",
      }}
    >
      {initials(name)}
    </span>
  );
}

function RefFace({ id, onRemove }: { id: number; onRemove: () => void }) {
  const { t } = useTranslation();
  const img = useQuery({
    queryKey: ["identity-face", id],
    queryFn: () => commands.identityFaceImage(id),
    enabled: isTauri(),
    staleTime: Infinity,
  });
  return (
    <span style={{ position: "relative", display: "inline-block" }}>
      {img.data ? (
        <img src={img.data} alt="" width={64} height={64} style={{ borderRadius: 6 }} />
      ) : (
        <span
          style={{
            display: "block",
            width: 64,
            height: 64,
            borderRadius: 6,
            background: "var(--pc-thumb-bg)",
          }}
        />
      )}
      <button
        type="button"
        className="btn icon-btn"
        aria-label={t("identities.removeFace")}
        onClick={onRemove}
        style={{ position: "absolute", top: 2, right: 2, minWidth: 22, minHeight: 22 }}
      >
        ×
      </button>
    </span>
  );
}

type Draft = { name: string; shortName: string; group: string; aliases: string; threshold: number };

const toDraft = (r: IdentityRow | null, def: number): Draft => ({
  name: r?.name ?? "",
  shortName: r?.shortName ?? "",
  group: r?.group ?? "",
  aliases: r?.aliases.join(", ") ?? "",
  threshold: r?.threshold ?? def,
});

const toInput = (d: Draft, def: number): IdentityInput => ({
  name: d.name.trim(),
  shortName: d.shortName.trim() || null,
  group: d.group.trim() || null,
  threshold: Math.abs(d.threshold - def) < 1e-6 ? null : d.threshold,
  aliases: d.aliases
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean),
});

function Editor({
  row,
  creating,
  all,
  groups,
  defaultThreshold,
  onSaved,
  onDeleted,
}: {
  row: IdentityRow | null;
  creating: boolean;
  all: IdentityRow[];
  groups: string[];
  defaultThreshold: number;
  onSaved: (uid: string) => void;
  onDeleted: () => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [draft, setDraft] = useState<Draft>(toDraft(row, defaultThreshold));
  const [error, setError] = useState("");
  const [choices, setChoices] = useState<{ path: string; faces: FaceChoice[] } | null>(null);
  const [fromLibrary, setFromLibrary] = useState(false);
  const [mergeInto, setMergeInto] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    setDraft(toDraft(row, defaultThreshold));
    setError("");
    setChoices(null);
    setFromLibrary(false);
    setMergeInto(null);
    setConfirmDelete(false);
    setNotice("");
  }, [row?.uid, creating, defaultThreshold]); // eslint-disable-line react-hooks/exhaustive-deps
  const faces = useQuery({
    queryKey: ["identity-faces", row?.uid],
    queryFn: () => commands.identityFaces(row!.uid),
    enabled: isTauri() && !!row,
  });
  const libraryFaces = useQuery({
    queryKey: ["faces", row?.person],
    queryFn: () => commands.personFaces(row!.person),
    enabled: isTauri() && fromLibrary && row?.person != null,
  });
  const refresh = () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: KEY }),
      qc.invalidateQueries({ queryKey: ["identity-faces"] }),
      qc.invalidateQueries({ queryKey: ["people"] }),
    ]);
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await refresh();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const save = () =>
    run(async () => {
      const uid = await commands.saveIdentity(
        creating ? null : row!.uid,
        toInput(draft, defaultThreshold),
      );
      onSaved(uid);
    });
  const addFile = async (path: string, choice: number | null) => {
    if (!row) return;
    await run(async () => {
      const out = await commands.addIdentityReferenceFile(row.uid, path, choice);
      if (out.outcome === "choose") setChoices({ path, faces: out.faces });
      else {
        setChoices(null);
        if (out.outcome === "noFace") setNotice(t("identities.noFace"));
      }
    });
  };
  const pickFile = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Immagini",
          extensions: ["jpg", "jpeg", "png", "heic", "heif", "tif", "tiff", "webp"],
        },
      ],
    });
    if (typeof path === "string") await addFile(path, null);
  };
  const field = (key: keyof Draft, label: string, extra: Record<string, unknown> = {}) => (
    <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span style={{ fontWeight: 600 }}>{label}</span>
      <input
        className="field"
        value={String(draft[key])}
        onChange={(e) => setDraft({ ...draft, [key]: e.target.value })}
        style={{ minHeight: 36 }}
        {...extra}
      />
    </label>
  );
  if (!row && !creating) return <p className="muted">{t("identities.none")}</p>;
  const title = creating ? t("identities.new") : row!.name;
  return (
    <>
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <Avatar name={draft.name || "?"} size={48} />
        <div>
          <div className="muted" style={{ fontSize: 12 }}>
            {row ? t("identities.createdAt", { date: fmtDate(new Date(row.createdAt)) }) : ""}
          </div>
          <strong style={{ fontSize: 20 }}>{title}</strong>
        </div>
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
        style={{ display: "flex", flexDirection: "column", gap: 10 }}
      >
        {field("name", t("identities.name"), { required: true })}
        {field("shortName", t("identities.shortName"), {
          placeholder: draft.name.trim().split(/\s+/)[0] ?? "",
        })}
        {field("group", t("identities.group"), { list: "pc-identity-groups" })}
        <datalist id="pc-identity-groups">
          {groups.map((g) => (
            <option key={g} value={g} />
          ))}
        </datalist>
        {field("aliases", t("identities.aliases"), { placeholder: t("identities.aliasesHint") })}
        <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <span style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ fontWeight: 600 }}>{t("identities.threshold")}</span>
            <span className="mono">{draft.threshold.toFixed(2)}</span>
          </span>
          <input
            type="range"
            min={0}
            max={100}
            value={Math.round(draft.threshold * 100)}
            aria-label={t("identities.threshold")}
            onChange={(e) => setDraft({ ...draft, threshold: Number(e.target.value) / 100 })}
          />
        </label>
        <button className="btn btn-primary" type="submit" disabled={!draft.name.trim()}>
          {creating ? t("identities.create") : t("identities.save")}
        </button>
      </form>
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {row && (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <h2 style={{ margin: 0, fontSize: 16 }}>
              {t("identities.references", { count: faces.data?.length ?? row.faces })}
            </h2>
            <button className="btn" style={{ marginLeft: "auto" }} onClick={() => void pickFile()}>
              {t("identities.addFromFile")}
            </button>
            {row.person != null && (
              <button className="btn" onClick={() => setFromLibrary((v) => !v)}>
                {t("identities.addFromLibrary")}
              </button>
            )}
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {faces.data?.map((f) => (
              <RefFace
                key={f.id}
                id={f.id}
                onRemove={() => void run(() => commands.removeIdentityFace(f.id))}
              />
            ))}
          </div>
          <span className="muted" style={{ fontSize: 12 }}>
            {t("identities.referencesHint")}
          </span>
          {choices && (
            <div role="group" aria-label={t("identities.chooseFace")} className="feature-card">
              <span>{t("identities.chooseFace")}</span>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                {choices.faces.map((f) => (
                  <button
                    key={f.index}
                    type="button"
                    className="btn"
                    style={{ padding: 2 }}
                    aria-label={t("identities.faceN", { n: f.index + 1 })}
                    onClick={() => void addFile(choices.path, f.index)}
                  >
                    <img src={f.crop} alt="" width={72} height={72} />
                  </button>
                ))}
              </div>
            </div>
          )}
          {fromLibrary && (
            <div className="feature-card" style={{ alignItems: "stretch" }}>
              <span>{t("identities.chooseLibraryFace")}</span>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                {libraryFaces.data?.slice(0, 24).map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    className="btn"
                    style={{ padding: 2 }}
                    aria-label={t("identities.libraryFaceN", { id: f.id })}
                    onClick={() =>
                      void run(async () => {
                        await commands.addIdentityReferenceFace(row.uid, f.id);
                        setFromLibrary(false);
                      })
                    }
                  >
                    <FaceCrop face={f} />
                  </button>
                ))}
              </div>
            </div>
          )}
          <div style={{ color: "var(--pc-text-soft)", lineHeight: 1.5 }}>
            {row.person != null ? (
              <>
                {t("identities.linked", { count: row.photos })}{" "}
                <a
                  href="#"
                  onClick={(e) => {
                    e.preventDefault();
                    useLayout.getState().setModule("people");
                  }}
                >
                  {t("identities.openPeople")}
                </a>
              </>
            ) : (
              t("identities.notLinked")
            )}
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {mergeInto === null ? (
              <button className="btn" onClick={() => setMergeInto("")}>
                {t("identities.mergeWith")}
              </button>
            ) : (
              <div className="feature-card" style={{ alignItems: "stretch" }}>
                <select
                  className="field"
                  aria-label={t("identities.mergeTarget")}
                  value={mergeInto}
                  onChange={(e) => setMergeInto(e.target.value)}
                >
                  <option value="">{t("identities.mergeTarget")}</option>
                  {all
                    .filter((x) => x.uid !== row.uid)
                    .map((x) => (
                      <option key={x.uid} value={x.uid}>
                        {x.name}
                      </option>
                    ))}
                </select>
                {mergeInto && (
                  <p style={{ margin: 0 }}>
                    {t("identities.mergeConfirm", {
                      from: row.name,
                      into: all.find((x) => x.uid === mergeInto)?.name ?? "",
                    })}
                  </p>
                )}
                <div className="feature-actions" style={{ margin: 0 }}>
                  <button
                    className="btn btn-primary"
                    disabled={!mergeInto}
                    onClick={() =>
                      void run(async () => {
                        await commands.mergeIdentities(row.uid, mergeInto);
                        onSaved(mergeInto);
                      })
                    }
                  >
                    {t("identities.merge")}
                  </button>
                  <button className="btn" onClick={() => setMergeInto(null)}>
                    {t("identities.cancel")}
                  </button>
                </div>
              </div>
            )}
            {confirmDelete ? (
              <div
                role="alertdialog"
                aria-label={t("identities.deleteConfirm", { name: row.name })}
                className="feature-card"
                style={{ alignItems: "stretch" }}
              >
                <p style={{ margin: 0 }}>{t("identities.deleteConfirm", { name: row.name })}</p>
                <div className="feature-actions" style={{ margin: 0 }}>
                  <button
                    className="btn btn-danger"
                    onClick={() =>
                      void run(async () => {
                        await commands.deleteIdentity(row.uid);
                        onDeleted();
                      })
                    }
                  >
                    {t("identities.confirm")}
                  </button>
                  <button className="btn" onClick={() => setConfirmDelete(false)}>
                    {t("identities.cancel")}
                  </button>
                </div>
              </div>
            ) : (
              <button className="btn btn-danger" onClick={() => setConfirmDelete(true)}>
                {t("identities.delete")}
              </button>
            )}
          </div>
        </>
      )}
    </>
  );
}

function Transfer({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [includeFaces, setIncludeFaces] = useState(false);
  const [preview, setPreview] = useState<{ path: string; p: ImportPreview } | null>(null);
  const [merge, setMerge] = useState<string[]>([]);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const exportFile = () =>
    run(async () => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: "photo-cat-identita.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await commands.exportIdentities(path, includeFaces);
      setMessage(t("identities.exported"));
    });
  const importFile = () =>
    run(async () => {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof path !== "string") return;
      const p = await commands.previewIdentityImport(path);
      setPreview({ path, p });
      setMerge(p.conflicts.map((c) => c.name));
    });
  const apply = () =>
    run(async () => {
      if (!preview) return;
      const out = await commands.importIdentities(preview.path, merge);
      setPreview(null);
      setMessage(
        t("identities.imported", {
          created: out.created,
          merged: out.merged,
          skipped: out.skipped,
        }),
      );
      await qc.invalidateQueries({ queryKey: KEY });
      onDone();
    });
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="feature-actions" style={{ margin: 0 }}>
        <button className="btn" onClick={() => void importFile()}>
          {t("identities.import")}
        </button>
        <button className="btn" onClick={() => void exportFile()}>
          {t("identities.export")}
        </button>
        <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input
            type="checkbox"
            checked={includeFaces}
            onChange={(e) => setIncludeFaces(e.target.checked)}
          />
          {t("identities.includeFaces")}
        </label>
      </div>
      {preview && (
        <div
          className="feature-card"
          style={{ alignItems: "stretch" }}
          role="group"
          aria-label={t("identities.importPreview")}
        >
          <strong>{t("identities.importPreview")}</strong>
          <span>{t("identities.importNew", { count: preview.p.new.length })}</span>
          {preview.p.conflicts.map((c) => (
            <label key={c.name} style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input
                type="checkbox"
                checked={merge.includes(c.name)}
                onChange={(e) =>
                  setMerge((m) =>
                    e.target.checked ? [...m, c.name] : m.filter((x) => x !== c.name),
                  )
                }
              />
              {t("identities.importMerge", { name: c.name })}
            </label>
          ))}
          <div className="feature-actions" style={{ margin: 0 }}>
            <button className="btn btn-primary" onClick={() => void apply()}>
              {t("identities.importApply")}
            </button>
            <button className="btn" onClick={() => setPreview(null)}>
              {t("identities.cancel")}
            </button>
          </div>
        </div>
      )}
      {message && <p role="status">{message}</p>}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}

export function IdentitiesPage() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [group, setGroup] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const view = useQuery({
    queryKey: [...KEY, search.trim()],
    queryFn: () => commands.listIdentities(search.trim() || null),
    enabled: isTauri(),
  });
  const all = useQuery({
    queryKey: [...KEY, ""],
    queryFn: () => commands.listIdentities(null),
    enabled: isTauri(),
  });
  const v = view.data;
  const everyone = all.data?.identities ?? [];
  const groups = all.data?.groups ?? [];
  const rows = useMemo(
    () => (v?.identities ?? []).filter((r) => group === null || r.group === group),
    [v, group],
  );
  const row = everyone.find((r) => r.uid === selected) ?? null;
  const def = v?.defaultThreshold ?? 0.363;
  const grid = "52px minmax(0, 1.4fr) minmax(0, 1.2fr) 110px 120px 140px";
  return (
    <div style={{ flex: 1, display: "flex", flexWrap: "wrap", minWidth: 0, overflow: "auto" }}>
      <section
        style={{
          flex: "999 1 560px",
          minWidth: 0,
          padding: 24,
          display: "flex",
          flexDirection: "column",
          gap: 16,
        }}
      >
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
          <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("identities.title")}</h1>
          <span className="muted">
            {t("identities.summary", { count: everyone.length, groups: groups.length })}
          </span>
          <button
            className="btn btn-primary"
            style={{ marginLeft: "auto" }}
            onClick={() => {
              setSelected(null);
              setCreating(true);
            }}
          >
            {t("identities.newButton")}
          </button>
        </div>
        <Transfer onDone={() => {}} />
        <div
          className="feature-card"
          role="note"
          style={{
            flexDirection: "row",
            alignItems: "flex-start",
            borderColor: "var(--pc-selection-border)",
          }}
        >
          <span>
            {t("identities.appNote")}
            {all.data?.storePath && (
              <>
                {" "}
                {t("identities.storeAt")} <code className="mono">{all.data.storePath}</code>
              </>
            )}
          </span>
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center" }}>
          <button
            className="pill"
            aria-pressed={group === null}
            onClick={() => setGroup(null)}
            style={{ borderColor: group === null ? "var(--pc-accent)" : undefined }}
          >
            {t("identities.allGroups", { count: everyone.length })}
          </button>
          {groups.map((g) => (
            <button
              key={g}
              className="pill"
              aria-pressed={group === g}
              onClick={() => setGroup(g)}
              style={{ borderColor: group === g ? "var(--pc-accent)" : undefined }}
            >
              {g} ({everyone.filter((r) => r.group === g).length})
            </button>
          ))}
          <input
            type="search"
            className="field"
            aria-label={t("identities.search")}
            placeholder={t("identities.search")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            style={{ marginLeft: "auto", width: 220 }}
          />
        </div>
        <div
          role="table"
          aria-label={t("identities.title")}
          style={{ border: "1px solid var(--pc-border)", borderRadius: 8, overflow: "hidden" }}
        >
          <div
            role="row"
            className="muted"
            style={{
              display: "grid",
              gridTemplateColumns: grid,
              gap: 12,
              padding: "8px 14px",
              background: "var(--pc-panel)",
              fontSize: 12,
              textTransform: "uppercase",
              letterSpacing: 0.6,
            }}
          >
            <span role="columnheader" />
            <span role="columnheader">{t("identities.colName")}</span>
            <span role="columnheader">{t("identities.colAliases")}</span>
            <span role="columnheader">{t("identities.colGroup")}</span>
            <span role="columnheader">{t("identities.colRefs")}</span>
            <span role="columnheader">{t("identities.colLibrary")}</span>
          </div>
          {rows.map((r) => (
            <button
              key={r.uid}
              type="button"
              role="row"
              aria-selected={r.uid === selected}
              onClick={() => {
                setCreating(false);
                setSelected(r.uid);
              }}
              style={{
                display: "grid",
                gridTemplateColumns: grid,
                gap: 12,
                alignItems: "center",
                width: "100%",
                padding: "6px 14px",
                minHeight: 48,
                border: 0,
                borderTop: "1px solid var(--pc-raised)",
                textAlign: "left",
                cursor: "pointer",
                color: "var(--pc-text)",
                background: r.uid === selected ? "var(--pc-selection-bg)" : "transparent",
              }}
            >
              <span role="cell">
                <Avatar name={r.name} />
              </span>
              <span role="cell" style={{ fontWeight: 600 }}>
                {r.name}
              </span>
              <span role="cell" className="muted">
                {r.aliases.join(", ") || "—"}
              </span>
              <span role="cell">{r.group ?? "—"}</span>
              <span role="cell" style={{ color: r.faces ? undefined : "var(--pc-warn)" }}>
                {r.faces ? t("identities.refCount", { count: r.faces }) : t("identities.noRefs")}
              </span>
              <span role="cell" className="mono" style={{ fontSize: 13 }}>
                {r.person != null ? fmtInt(r.photos) : t("identities.notLinkedShort")}
              </span>
            </button>
          ))}
          {!rows.length && (
            <p className="muted" style={{ padding: 14, margin: 0 }}>
              {t("identities.empty")}
            </p>
          )}
        </div>
      </section>
      <aside
        aria-label={t("identities.edit")}
        style={{
          flex: "1 1 320px",
          maxWidth: 380,
          padding: 20,
          background: "var(--pc-panel)",
          borderLeft: "1px solid var(--pc-border)",
          display: "flex",
          flexDirection: "column",
          gap: 14,
        }}
      >
        <Editor
          row={creating ? null : row}
          creating={creating}
          all={everyone}
          groups={groups}
          defaultThreshold={def}
          onSaved={(uid) => {
            setCreating(false);
            setSelected(uid);
          }}
          onDeleted={() => setSelected(null)}
        />
      </aside>
    </div>
  );
}
