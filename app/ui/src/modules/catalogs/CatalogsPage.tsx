import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type Catalog } from "../../lib/ipc";
import { emptyFilter, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { RuleEditor, newRule, type RuleNode } from "./RuleEditor";
export function CatalogsPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [id, setId] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [smart, setSmart] = useState(false);
  const [rule, setRule] = useState<RuleNode>({ op: "and", rules: [newRule()] });
  const [error, setError] = useState("");
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const preview = useQuery({
    queryKey: ["catalog-preview", rule],
    queryFn: () => commands.libraryCount({ ...emptyFilter(), rule }),
    enabled: isTauri() && editing && smart,
    retry: false,
  });
  const edit = (c?: Catalog) => {
    setId(c?.id ?? null);
    setName(c?.name ?? "");
    setSmart(c?.kind === "smart");
    setRule((c?.rules as RuleNode | null) ?? { op: "and", rules: [newRule()] });
    setEditing(true);
    setError("");
  };
  const save = async () => {
    try {
      await commands.saveCatalog(id, name, smart ? "smart" : "manual", smart ? rule : null);
      setEditing(false);
      await qc.invalidateQueries({ queryKey: ["catalogs"] });
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  return (
    <section style={{ padding: 24, overflow: "auto", flex: 1 }}>
      <h1>{t("catalogs.title")}</h1>
      {error && <p role="alert">{error}</p>}
      <button className="btn" onClick={() => edit()}>
        {t("catalogs.create")}
      </button>
      {editing && (
        <div className="feature-card" style={{ alignItems: "stretch", marginTop: 16 }}>
          <input
            className="field"
            aria-label={t("catalogs.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <label>
            <input type="checkbox" checked={smart} onChange={(e) => setSmart(e.target.checked)} />
            {t("catalogs.smart")}
          </label>
          {smart && (
            <>
              <RuleEditor rule={rule} onChange={setRule} />
              <p role="status">
                {preview.error
                  ? String((preview.error as { detail?: string }).detail ?? preview.error)
                  : t("catalogs.preview", { count: preview.data ?? 0 })}
              </p>
            </>
          )}
          <div className="feature-actions">
            <button
              className="btn"
              disabled={!name.trim() || (smart && preview.isError)}
              onClick={() => void save()}
            >
              {t("people.save")}
            </button>
            <button className="btn" onClick={() => setEditing(false)}>
              {t("catalogs.cancel")}
            </button>
          </div>
        </div>
      )}
      <div className="feature-grid">
        {catalogs.data?.map(({ catalog: c, items }) => (
          <article className="feature-card" key={c.id}>
            <h2>{c.name}</h2>
            <p>
              {c.kind === "smart" ? t("catalogs.smart") : t("catalogs.manual")} · {items}
            </p>
            <button
              className="btn"
              onClick={() => {
                useLibrary.getState().resetFilter();
                useLibrary.getState().setFilter({ catalogId: c.id });
                useLayout.getState().setModule("library");
              }}
            >
              {t("catalogs.open")}
            </button>
            <button className="btn" onClick={() => edit(c)}>
              {t("catalogs.edit")}
            </button>
            <button
              className="btn"
              onClick={() => {
                if (window.confirm(t("catalogs.deleteConfirm", { count: items })))
                  void commands
                    .deleteCatalog(c.id, true)
                    .then(() => qc.invalidateQueries({ queryKey: ["catalogs"] }))
                    .catch((e) => setError(String(e)));
              }}
            >
              {t("catalogs.delete")}
            </button>
          </article>
        ))}
      </div>
    </section>
  );
}
