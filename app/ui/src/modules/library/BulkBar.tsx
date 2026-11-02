import { BusyButton } from "../../components/feedback/BusyButton";
import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type BulkAction } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useLive } from "../../store/live";
import { CategoryPicker } from "./CategoryPicker";
import { useLabelName, useTaxonomy } from "./labelName";
import { useCatalogAction } from "./useCatalogAction";
export function BulkBar() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const catalogAction = useCatalogAction();
  const { selected, filter, setSelection } = useLibrary();
  const [action, setAction] = useState("add_tag");
  const [value, setValue] = useState("");
  const [group, setGroup] = useState("subject");
  const taxonomy = useTaxonomy();
  const names = useLabelName();
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
      void qc.invalidateQueries({ queryKey: ["facets"] });
      void qc.invalidateQueries({ queryKey: ["media-detail"] });
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
    const destructive = action === "remove_library";
    if (destructive && !window.confirm(t("bulk.confirm", { count: selected.length }))) return;
    let operation: BulkAction;
    switch (action) {
      case "add_tag":
      case "remove_tag":
        operation = { action, name: value };
        break;
      case "add_catalog":
        operation = { action, catalog: Number(value) };
        break;
      case "remove_catalog": {
        const cat = catalogs.data?.find((c) => c.catalog.id === Number(value));
        await catalogAction.act(selected, Number(value), cat?.catalog.name ?? "");
        return;
      }
      case "set_label":
        operation = { action, group, value: value || null };
        break;
      case "assign_person":
        operation = { action, person: Number(value) };
        break;
      case "set_place":
        operation = { action: "set_place", place: { name: value.trim() } };
        break;
      case "reset_place":
        operation = { action: "set_place", place: null };
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
    "set_place",
  ].includes(action);
  const done = result?.phases.reduce((n, p) => n + p.done, 0) ?? 0;
  return (
    <div style={{ padding: "8px 16px", borderBottom: "1px solid var(--pc-border)" }}>
      {(filter.catalogId != null || filter.excludedFrom != null) && (
        <BusyButton
          className="btn"
          disabled={busy || catalogAction.busy || !selected.length}
          onClick={async () => {
            const id = filter.excludedFrom ?? filter.catalogId!;
            const cat = catalogs.data?.find((c) => c.catalog.id === id);
            return await catalogAction.act(
              selected,
              id,
              cat?.catalog.name ?? "",
              filter.excludedFrom != null,
            );
          }}
        >
          {t(filter.excludedFrom != null ? "exclusions.restore" : "exclusions.exclude")}
        </BusyButton>
      )}
      <div className="feature-actions" style={{ margin: 0 }}>
        <BusyButton
          className="btn"
          disabled={busy}
          onClick={async () => await run(() => select(false))}
        >
          {t("bulk.selectAll")}
        </BusyButton>
        <BusyButton
          className="btn"
          disabled={busy}
          onClick={async () => await run(() => select(true))}
        >
          {t("bulk.invert")}
        </BusyButton>
        <BusyButton className="btn" onClick={() => setSelection([])}>
          {t("bulk.clear")}
        </BusyButton>
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
            "set_place",
            "reset_place",
            "reanalyze",
            "export",
            "organize",
            "remove_library",
          ].map((a) => (
            <option key={a} value={a}>
              {a === "remove_catalog" ? t("exclusions.exclude") : t(`bulk.actions.${a}`)}
            </option>
          ))}
        </select>
        {action === "set_label" && (
          <>
            <select
              className="field"
              aria-label={t("catalogs.field")}
              value={group}
              onChange={(e) => {
                setGroup(e.target.value);
                setValue("");
              }}
            >
              {(taxonomy.data?.groups ?? []).map((g) => (
                <option key={g.id} value={g.id}>
                  {names.group(g.id)}
                </option>
              ))}
            </select>
            <CategoryPicker
              ariaLabel={t("bulk.value")}
              group={taxonomy.data?.groups.find((g) => g.id === group)}
              value={value}
              onChange={setValue}
              onCreate={async (label) => {
                const r = await commands.addCategory(group, label, [], selected);
                await qc.invalidateQueries({ queryKey: ["taxonomy"] });
                setValue(r.id);
                if (r.job != null) setJob(r.job);
              }}
            />
          </>
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
              ?.filter((c) => action !== "add_catalog" || c.catalog.kind === "manual")
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
          ["add_tag", "remove_tag", "set_place"].includes(action) && (
            <input
              className="field"
              aria-label={t("bulk.value")}
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          )
        )}
        <BusyButton
          className="btn"
          disabled={busy || !selected.length || (needsValue && !value.trim())}
          onClick={async () => await run(submit)}
        >
          {t("bulk.apply")}
        </BusyButton>
        <BusyButton
          className="btn"
          disabled={busy || !undo.data}
          title={undo.data ?? ""}
          onClick={async () =>
            await run(async () => {
              await commands.undoBulk();
              await qc.invalidateQueries();
            })
          }
        >
          {t("bulk.undo")}
        </BusyButton>
      </div>
      {error && <p role="alert">{error}</p>}
      {result && (
        <div role="status">
          {t("bulk.outcome", { done, failed: result.failed })} ·{" "}
          {Math.round(result.overallPercent ?? 0)}%{" "}
          <BusyButton
            className="btn"
            onClick={() => useLayout.getState().openDiagnostics({ jobId: result.id })}
          >
            {t("bulk.details")}
          </BusyButton>
        </div>
      )}
    </div>
  );
}
