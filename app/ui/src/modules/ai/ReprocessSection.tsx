// Rianalisi (ai-workspace 3.4): ambito, fasi e modalità con anteprima per fase, poi avvio del job.
import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type ReprocessScope, type Stage, type Target } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { emptyFilter, selCount, selTarget, useLibrary } from "../../store/library";
import { notify, notifyError } from "../../store/toasts";
import { Button } from "../../components/ui";
import { activeChips } from "../library/chips";
import { useReprocessPreset } from "./reprocess/preset";

const STAGES: Stage[] = ["classification", "persons", "faces", "similar", "phash"];
const DEFAULT_STAGES: Stage[] = ["classification", "persons", "faces", "similar"];

type ScopeMode = "library" | "catalogs" | "selection" | "filter";

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

export function ReprocessSection() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const selection = useLibrary((s) => s.selection);
  const catalogs = useQuery({ queryKey: ["catalogs"], queryFn: () => commands.listCatalogs() });
  const [mode, setMode] = useState<ScopeMode>("library");
  const [catalogIds, setCatalogIds] = useState<number[]>([]);
  const [stages, setStages] = useState<Stage[]>(DEFAULT_STAGES);
  const [force, setForce] = useState(false);
  // Bersaglio fissato dall'azione "Rianalizza" della Libreria (può differire dalla selezione).
  const [fixed, setFixed] = useState<{ target: Target; count: number | null } | null>(null);

  useEffect(() => {
    const p = useReprocessPreset.getState();
    if (p.scope?.kind === "target") {
      setFixed({ target: p.scope.target, count: p.count });
      setMode("selection");
    } else if (p.scope?.kind === "catalogs") {
      setCatalogIds(p.scope.ids);
      setMode("catalogs");
    }
    p.clear();
  }, []);

  const userCatalogs = (catalogs.data ?? []).filter((c) => !c.catalog.system);
  const liveCount = selCount(selection);
  const selectionTarget: Target | null = fixed
    ? fixed.target
    : liveCount > 0
      ? selTarget(selection)
      : null;
  const selectionCount = fixed ? fixed.count : liveCount;
  const filterActive = activeChips({ ...filter, catalogId: null }, t as never).length > 0;
  const filterBase = useQuery({
    queryKey: ["ai-reprocess-filter", filter],
    enabled: filterActive,
    queryFn: () => commands.librarySelection({ ...emptyFilter(), ...filter }),
  });
  const filterCount = filterBase.data?.count ?? 0;
  const filterOk = filterActive && filterCount > 0;

  const scope: ReprocessScope | null = useMemo(() => {
    if (mode === "library") return { kind: "library" };
    if (mode === "catalogs")
      return catalogIds.length ? { kind: "catalogs", ids: catalogIds } : null;
    if (mode === "selection")
      return selectionTarget ? { kind: "target", target: selectionTarget } : null;
    if (filterOk && filterBase.data)
      return {
        kind: "target",
        target: { kind: "filter", filter, maxId: filterBase.data.maxId, except: [] },
      };
    return null;
  }, [mode, catalogIds, selectionTarget, filterOk, filterBase.data, filter]);

  const key = scope ? JSON.stringify([scope, stages, force]) : "";
  const debounced = useDebounced(key, 250);
  const stable = debounced === key && key !== "";
  const preview = useQuery({
    queryKey: ["ai-reprocess-preview", debounced],
    enabled: stable,
    queryFn: () => commands.aiReprocessPreview(scope!, stages, force),
  });
  const counts = stable ? preview.data : undefined;
  const countOf = (s: Stage) => counts?.find((c) => c.stage === s);
  const usable = stages.filter((s) => countOf(s)?.available !== false);
  const total = counts ? usable.reduce((n, s) => n + (countOf(s)?.count ?? 0), 0) : 0;
  const canStart = !!scope && !!counts && total > 0;

  const toggleStage = (s: Stage) =>
    setStages((cur) => (cur.includes(s) ? cur.filter((x) => x !== s) : [...cur, s]));
  const toggleCatalog = (id: number) =>
    setCatalogIds((cur) => (cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]));

  const start = async () => {
    if (!scope) return;
    try {
      const job = await commands.aiReprocess(scope, usable, force);
      notify({
        message: t("aiReprocess.started"),
        action: {
          label: t("aiReprocess.seeJob"),
          run: () => useLayout.getState().openModule({ module: "jobs", target: job }),
        },
      });
    } catch (error) {
      notifyError(error);
    }
  };

  const photos = (n: number | null) =>
    n == null ? "" : ` (${t("aiReprocess.photos", { count: n })})`;

  return (
    <section className="ai-section" aria-labelledby="ai-section-title">
      <h2 id="ai-section-title">{t("ai.sections.reprocess")}</h2>
      <p className="muted">{t("aiReprocess.intro")}</p>
      <div className="ai-reprocess-form">
        <fieldset>
          <legend>{t("aiReprocess.scope")}</legend>
          <label>
            <input
              type="radio"
              name="reprocess-scope"
              checked={mode === "library"}
              onChange={() => setMode("library")}
            />{" "}
            {t("aiReprocess.scopeLibrary")}
          </label>
          <label>
            <input
              type="radio"
              name="reprocess-scope"
              checked={mode === "catalogs"}
              onChange={() => setMode("catalogs")}
            />{" "}
            {t("aiReprocess.scopeCatalogs")}
          </label>
          {mode === "catalogs" && (
            <div className="ai-reprocess-catalogs">
              {userCatalogs.length === 0 && (
                <span className="muted">{t("aiReprocess.noCatalogs")}</span>
              )}
              {userCatalogs.map((c) => (
                <label key={c.catalog.id}>
                  <input
                    type="checkbox"
                    checked={catalogIds.includes(c.catalog.id)}
                    onChange={() => toggleCatalog(c.catalog.id)}
                  />{" "}
                  {c.catalog.name}
                </label>
              ))}
            </div>
          )}
          <label>
            <input
              type="radio"
              name="reprocess-scope"
              checked={mode === "selection"}
              disabled={!selectionTarget}
              onChange={() => setMode("selection")}
            />{" "}
            {t("aiReprocess.scopeSelection")}
            {selectionTarget ? photos(selectionCount) : ""}
          </label>
          <label>
            <input
              type="radio"
              name="reprocess-scope"
              checked={mode === "filter"}
              disabled={!filterOk}
              onChange={() => setMode("filter")}
            />{" "}
            {t("aiReprocess.scopeFilter")}
            {filterOk ? photos(filterCount) : ""}
          </label>
        </fieldset>

        <fieldset>
          <legend>{t("aiReprocess.stages")}</legend>
          {STAGES.map((s) => {
            const off = countOf(s)?.available === false;
            return (
              <label key={s} className={off ? "muted" : undefined}>
                <input
                  type="checkbox"
                  checked={stages.includes(s)}
                  disabled={off}
                  onChange={() => toggleStage(s)}
                />{" "}
                {t(`aiReprocess.stage.${s}`)}
              </label>
            );
          })}
        </fieldset>

        <fieldset>
          <legend>{t("aiReprocess.mode")}</legend>
          <label>
            <input
              type="radio"
              name="reprocess-mode"
              checked={!force}
              onChange={() => setForce(false)}
            />{" "}
            {t("aiReprocess.modeMissing")}
          </label>
          <label>
            <input
              type="radio"
              name="reprocess-mode"
              checked={force}
              onChange={() => setForce(true)}
            />{" "}
            {t("aiReprocess.modeForce")}
          </label>
          <p className="muted">{t("aiReprocess.forceNote")}</p>
        </fieldset>

        <div>
          <h3>{t("aiReprocess.preview")}</h3>
          {!scope ? (
            <p className="muted">
              {mode === "catalogs" ? t("aiReprocess.pickScope") : t("aiReprocess.none")}
            </p>
          ) : preview.isError ? (
            <p role="alert">{t("aiReprocess.previewError")}</p>
          ) : !counts ? (
            <p className="muted">{t("aiReprocess.previewing")}</p>
          ) : (
            <table className="ai-reprocess-table">
              <thead>
                <tr>
                  <th>{t("aiReprocess.previewStage")}</th>
                  <th>{t("aiReprocess.previewCount")}</th>
                </tr>
              </thead>
              <tbody>
                {counts.map((c) => (
                  <tr key={c.stage} className={c.available ? undefined : "is-off"}>
                    <td>{t(`aiReprocess.stage.${c.stage}`)}</td>
                    <td>
                      {c.available ? (
                        fmtInt(c.count)
                      ) : (
                        <>
                          {t("aiReprocess.unavailable")}{" "}
                          <button
                            type="button"
                            className="link-button"
                            onClick={() =>
                              useLayout
                                .getState()
                                .openModule({ module: "settings", section: "models" })
                            }
                          >
                            {t("aiReprocess.openModels")}
                          </button>
                        </>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {counts && total === 0 && <p className="muted">{t("aiReprocess.none")}</p>}
        </div>

        <div>
          <Button variant="primary" disabled={!canStart} onClick={() => void start()}>
            {t("aiReprocess.start")}
          </Button>
        </div>
      </div>
    </section>
  );
}
