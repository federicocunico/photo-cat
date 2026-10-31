import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type BulkAction } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
export function BulkBar() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { selected, filter, setSelection } = useLibrary();
  const [action, setAction] = useState("add_tag");
  const [value, setValue] = useState("");
  const [group, setGroup] = useState("location");
  const [error, setError] = useState("");
  const [job, setJob] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const result = useLive((s) => (job === null ? undefined : s.jobs[job]));
  const undo = useQuery({
    queryKey: ["bulk-undo"],
    queryFn: commands.lastBulkUndo,
    enabled: isTauri(),
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const people = useQuery({
    queryKey: ["people"],
    queryFn: commands.listPeople,
    enabled: isTauri() && action === "assign_person",
  });
  useEffect(() => {
    if (
      result &&
      ["completed", "completed_with_errors", "cancelled", "failed"].includes(result.state)
    ) {
      void qc.invalidateQueries({ queryKey: ["library"] });
      void qc.invalidateQueries({ queryKey: ["library-count"] });
      void qc.invalidateQueries({ queryKey: ["bulk-undo"] });
      void qc.invalidateQueries({ queryKey: ["catalogs"] });
    }
  }, [result, qc]);
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const select = async (invert: boolean) => {
    const ids = await commands.libraryIds(filter);
    const s = new Set(selected);
    setSelection(invert ? ids.filter((id) => !s.has(id)) : ids);
  };
  const submit = async () => {
    if (action === "reanalyze") {
      setJob(await commands.reanalyze(selected));
      return;
    }
    if (action === "export" || action === "organize") {
      useLayout.getState().setModule("organize");
      return;
    }
    const destructive = action === "remove_library" || action === "remove_catalog";
    if (destructive && !window.confirm(t("bulk.confirm", { count: selected.length }))) return;
    let operation: BulkAction;
    switch (action) {
      case "add_tag":
      case "remove_tag":
        operation = { action, name: value };
        break;
      case "add_catalog":
      case "remove_catalog":
        operation = { action, catalog: Number(value) };
        break;
      case "set_label":
        operation = { action, group, value: value || null };
        break;
      case "assign_person":
        operation = { action, person: Number(value) };
        break;
      default:
        operation = { action: "remove_library" };
    }
    setJob(await commands.startBulk(selected, operation, destructive));
  };
  const needsValue = [
    "add_tag",
    "remove_tag",
    "add_catalog",
    "remove_catalog",
    "assign_person",
  ].includes(action);
  const done = result?.phases.reduce((n, p) => n + p.done, 0) ?? 0;
  return (
    <div style={{ padding: "8px 16px", borderBottom: "1px solid var(--pc-border)" }}>
      <div className="feature-actions" style={{ margin: 0 }}>
        <button className="btn" disabled={busy} onClick={() => void run(() => select(false))}>
          {t("bulk.selectAll")}
        </button>
        <button className="btn" disabled={busy} onClick={() => void run(() => select(true))}>
          {t("bulk.invert")}
        </button>
        <button className="btn" onClick={() => setSelection([])}>
          {t("bulk.clear")}
        </button>
        <span>{t("people.selected", { count: selected.length })}</span>
        <select
          className="field"
          aria-label={t("bulk.action")}
          value={action}
          onChange={(e) => {
            setAction(e.target.value);
            setValue("");
          }}
        >
          {[
            "add_tag",
            "remove_tag",
            "add_catalog",
            "remove_catalog",
            "set_label",
            "assign_person",
            "reanalyze",
            "export",
            "organize",
            "remove_library",
          ].map((a) => (
            <option key={a} value={a}>
              {t(`bulk.actions.${a}`)}
            </option>
          ))}
        </select>
        {action === "set_label" && (
          <select
            className="field"
            aria-label={t("catalogs.field")}
            value={group}
            onChange={(e) => setGroup(e.target.value)}
          >
            {["subject", "location", "environment"].map((g) => (
              <option key={g}>{g}</option>
            ))}
          </select>
        )}
        {action.includes("catalog") ? (
          <select
            className="field"
            aria-label={t("bulk.value")}
            value={value}
            onChange={(e) => setValue(e.target.value)}
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
        ) : action === "assign_person" ? (
          <select
            className="field"
            aria-label={t("bulk.value")}
            value={value}
            onChange={(e) => setValue(e.target.value)}
          >
            <option value="">{t("people.target")}</option>
            {people.data?.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name ?? p.id}
              </option>
            ))}
          </select>
        ) : (
          ["add_tag", "remove_tag", "set_label"].includes(action) && (
            <input
              className="field"
              aria-label={t("bulk.value")}
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          )
        )}
        <button
          className="btn"
          disabled={busy || !selected.length || (needsValue && !value.trim())}
          onClick={() => void run(submit)}
        >
          {t("bulk.apply")}
        </button>
        <button
          className="btn"
          disabled={busy || !undo.data}
          title={undo.data ?? ""}
          onClick={() =>
            void run(async () => {
              await commands.undoBulk();
              await qc.invalidateQueries();
            })
          }
        >
          {t("bulk.undo")}
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
      {result && (
        <div role="status">
          {t("bulk.outcome", { done, failed: result.failed })} ·{" "}
          {Math.round(result.overallPercent ?? 0)}%{" "}
          <button
            className="btn"
            onClick={() => useLayout.getState().openDiagnostics({ jobId: result.id })}
          >
            {t("bulk.details")}
          </button>
        </div>
      )}
    </div>
  );
}
