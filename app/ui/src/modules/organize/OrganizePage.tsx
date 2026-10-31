import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Scope,
  type Method,
  type Preview,
  type Preflight,
} from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { fmtBytes } from "../../lib/format";
const LEVELS = [
  "place",
  "year",
  "month",
  "person",
  "subject",
  "location",
  "environment",
  "tag",
  "catalog",
];
export function OrganizePage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { filter, selected } = useLibrary();
  const [scopeName, setScope] = useState("selection");
  const [catalog, setCatalog] = useState("");
  const [destination, setDestination] = useState("");
  const [exportPath, setExportPath] = useState("");
  const [format, setFormat] = useState<"csv" | "json">("csv");
  const [separator, setSeparator] = useState(";");
  const [bom, setBom] = useState(true);
  const [fields, setFields] = useState<string[]>([]);
  const [method, setMethod] = useState<Method>("copy");
  const [levels, setLevels] = useState(["place", "year"]);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<{ key: string; data: Preview } | null>(null);
  const [check, setCheck] = useState<{ key: string; data: Preflight } | null>(null);
  const allFields = useQuery({
    queryKey: ["export-fields"],
    queryFn: commands.exportFields,
    enabled: isTauri(),
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const history = useQuery({
    queryKey: ["organizations"],
    queryFn: commands.listOrganizations,
    enabled: isTauri(),
    refetchInterval: 5000,
  });
  useEffect(() => {
    if (allFields.data) setFields(allFields.data);
  }, [allFields.data]);
  const scope: Scope =
    scopeName === "library"
      ? { scope: "library" }
      : scopeName === "catalog"
        ? { scope: "catalog", id: Number(catalog) }
        : scopeName === "filter"
          ? { scope: "filter", filter }
          : { scope: "selection", ids: selected };
  const options = { scope, destination, levels, method };
  const key = JSON.stringify(options);
  const valid = scopeName !== "selection" || selected.length > 0;
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    setNotice("");
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const pick = async (folder: boolean) => {
    if (!isTauri()) return;
    const dialog = await import("@tauri-apps/plugin-dialog");
    const path = folder
      ? await dialog.open({ directory: true, multiple: false })
      : await dialog.save({
          defaultPath: `catalog.${format}`,
          filters: [{ name: format.toUpperCase(), extensions: [format] }],
        });
    if (typeof path === "string") {
      if (folder) setDestination(path);
      else setExportPath(path);
    }
  };
  const queued = (id: number) => setNotice(t("organize.queued", { id }));
  return (
    <section style={{ padding: 24, overflow: "auto", flex: 1 }}>
      <h1>{t("organize.title")}</h1>
      {error && <p role="alert">{error}</p>}
      {notice && (
        <p role="status">
          {notice}{" "}
          <button className="btn" onClick={() => useLayout.getState().setModule("jobs")}>
            {t("organize.jobs")}
          </button>
        </p>
      )}
      <div className="feature-actions">
        <label>
          {t("organize.scope")}{" "}
          <select className="field" value={scopeName} onChange={(e) => setScope(e.target.value)}>
            {["library", "catalog", "selection", "filter"].map((s) => (
              <option key={s} value={s}>
                {t(`organize.scopes.${s}`)}
                {s === "selection" ? ` (${selected.length})` : ""}
              </option>
            ))}
          </select>
        </label>
        {scopeName === "catalog" && (
          <select
            className="field"
            aria-label={t("catalogs.title")}
            value={catalog}
            onChange={(e) => setCatalog(e.target.value)}
          >
            <option value="">—</option>
            {catalogs.data?.map(({ catalog: c }) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className="feature-card" style={{ alignItems: "stretch" }}>
        <h2>{t("organize.export")}</h2>
        <div className="feature-actions">
          <select
            className="field"
            aria-label={t("organize.format")}
            value={format}
            onChange={(e) => setFormat(e.target.value as "csv" | "json")}
          >
            <option value="csv">CSV</option>
            <option value="json">JSON</option>
          </select>
          <label>
            {t("organize.separator")}{" "}
            <input
              className="field"
              value={separator}
              maxLength={1}
              style={{ width: 48 }}
              onChange={(e) => setSeparator(e.target.value)}
            />
          </label>
          <label>
            <input type="checkbox" checked={bom} onChange={(e) => setBom(e.target.checked)} />
            UTF-8 BOM
          </label>
        </div>
        <fieldset>
          <legend>{t("organize.fields")}</legend>
          <div className="feature-actions">
            {allFields.data?.map((f) => (
              <label key={f}>
                <input
                  type="checkbox"
                  checked={fields.includes(f)}
                  onChange={(e) =>
                    setFields((v) => (e.target.checked ? [...v, f] : v.filter((x) => x !== f)))
                  }
                />
                {t(`organize.fieldNames.${f}`, { defaultValue: f })}
              </label>
            ))}
          </div>
        </fieldset>
        <div className="feature-actions">
          <input
            className="field"
            aria-label={t("organize.exportPath")}
            value={exportPath}
            onChange={(e) => setExportPath(e.target.value)}
          />
          <button className="btn" onClick={() => void run(() => pick(false))}>
            {t("organize.browse")}
          </button>
          <button
            className="btn"
            disabled={
              busy ||
              !valid ||
              !exportPath ||
              !fields.length ||
              !separator ||
              (scopeName === "catalog" && !catalog)
            }
            onClick={() =>
              void run(async () =>
                queued(
                  await commands.startExport({
                    scope,
                    destination: exportPath,
                    format,
                    separator,
                    bom,
                    fields,
                  }),
                ),
              )
            }
          >
            {t("organize.export")}
          </button>
        </div>
      </div>
      <div className="feature-card" style={{ alignItems: "stretch", marginTop: 20 }}>
        <h2>{t("organize.folders")}</h2>
        <div className="feature-actions">
          <input
            className="field"
            aria-label={t("organize.destination")}
            value={destination}
            onChange={(e) => setDestination(e.target.value)}
          />
          <button className="btn" onClick={() => void run(() => pick(true))}>
            {t("organize.browse")}
          </button>
          <select
            className="field"
            aria-label={t("organize.method")}
            value={method}
            onChange={(e) => setMethod(e.target.value as Method)}
          >
            {["copy", "hardlink", "symlink"].map((m) => (
              <option value={m} key={m}>
                {t(`organize.methods.${m}`)}
              </option>
            ))}
          </select>
        </div>
        <ol>
          {levels.map((level, i) => (
            <li key={i} className="feature-actions">
              <select
                className="field"
                aria-label={`${t("organize.level")} ${i + 1}`}
                value={level}
                onChange={(e) => setLevels((s) => s.map((l, j) => (i === j ? e.target.value : l)))}
              >
                {LEVELS.map((l) => (
                  <option key={l} value={l}>
                    {t(`organize.levels.${l}`)}
                  </option>
                ))}
              </select>
              <button
                className="btn"
                disabled={i === 0}
                onClick={() =>
                  setLevels((s) => {
                    const v = [...s];
                    [v[i - 1], v[i]] = [v[i], v[i - 1]];
                    return v;
                  })
                }
              >
                ↑
              </button>
              <button
                className="btn"
                disabled={levels.length === 1}
                onClick={() => setLevels((s) => s.filter((_, j) => j !== i))}
              >
                ×
              </button>
            </li>
          ))}
        </ol>
        <div className="feature-actions">
          <button
            className="btn"
            disabled={levels.length >= 6}
            onClick={() => setLevels((s) => [...s, "year"])}
          >
            {t("organize.addLevel")}
          </button>
          <button
            className="btn"
            disabled={busy || !valid || !destination}
            onClick={() =>
              void run(async () =>
                setPreview({ key, data: await commands.organizePreview(options) }),
              )
            }
          >
            {t("organize.preview")}
          </button>
          <button
            className="btn"
            disabled={busy || !valid || !destination}
            onClick={() =>
              void run(async () =>
                setCheck({ key, data: await commands.organizePreflight(options) }),
              )
            }
          >
            {t("organize.verify")}
          </button>
        </div>
        {preview?.key === key && (
          <>
            <p>
              {t("organize.summary", {
                count: preview.data.files,
                conflicts: preview.data.conflicts,
                space: fmtBytes(method === "copy" ? preview.data.bytes : 0),
              })}
            </p>
            <ul className="mono" style={{ maxHeight: 260, overflow: "auto" }}>
              {Object.entries(preview.data.folders).map(([p, n]) => (
                <li key={p}>
                  {p} · {n}
                </li>
              ))}
            </ul>
          </>
        )}
        {check?.key === key && (
          <p role="status">
            {check.data.available ? t("organize.available") : t("organize.unavailable")}{" "}
            {check.data.reason && t(`organize.reasons.${check.data.reason}`)}{" "}
            {!check.data.available && t("organize.alternative")}{" "}
            {!check.data.available && (
              <button className="btn" onClick={() => setMethod(check.data.alternative)}>
                {t(`organize.methods.${check.data.alternative}`)}
              </button>
            )}
          </p>
        )}
        <button
          className="btn"
          disabled={busy || preview?.key !== key || check?.key !== key || !check?.data.available}
          onClick={() => void run(async () => queued(await commands.startOrganize(options)))}
        >
          {t("organize.start")}
        </button>
      </div>
      <h2>{t("organize.history")}</h2>
      {history.data
        ?.filter((o) => o.state !== "undone")
        .map((o) => (
          <div className="feature-actions" key={o.id}>
            <span>
              #{o.id} · {o.dest} · {o.entries}
            </span>
            <button
              className="btn"
              disabled={busy || o.state === "running"}
              onClick={() => {
                if (window.confirm(t("organize.undoConfirm", { count: o.entries })))
                  void run(async () => {
                    const [removed, kept] = await commands.undoOrganization(o.id, true);
                    setNotice(t("organize.undone", { removed, kept }));
                    await qc.invalidateQueries({ queryKey: ["organizations"] });
                  });
              }}
            >
              {t("organize.undo")}
            </button>
          </div>
        ))}
    </section>
  );
}
