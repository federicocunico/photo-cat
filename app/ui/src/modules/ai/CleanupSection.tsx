// Pulizia (ai-workspace 3.6): toglie un'etichetta automatica da un ambito, con anteprima,
// conferma e avviso "Annulla operazione" a fine job.
import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands } from "../../lib/ipc";
import { notifyError } from "../../store/toasts";
import { Button, ConfirmDialog, FormField } from "../../components/ui";
import { useLabelName } from "../library/labelName";
import { useCleanupPreset } from "./results/cleanupPreset";
import { scopeTarget } from "./cleanup/target";
import { useRejectLabels } from "./cleanup/useRejectLabels";

const LIBRARY = "library";

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

export function CleanupSection() {
  const { t } = useTranslation();
  const taxonomy = useQuery({ queryKey: ["taxonomy"], queryFn: () => commands.getTaxonomy() });
  const catalogs = useQuery({ queryKey: ["catalogs"], queryFn: () => commands.listCatalogs() });
  const [grp, setGrp] = useState("");
  const [value, setValue] = useState("");
  const [scope, setScope] = useState(LIBRARY);
  const [limit, setLimit] = useState(false);
  const [maxScore, setMaxScore] = useState("0.35");
  const [confirming, setConfirming] = useState(false);
  const { start, busy } = useRejectLabels();
  const name = useLabelName();

  // Preimpostazione dai Risultati: si applica una volta e si svuota.
  useEffect(() => {
    const p = useCleanupPreset.getState();
    if (p.grp) setGrp(p.grp);
    if (p.value) setValue(p.value);
    if (p.maxScore != null) {
      setLimit(true);
      setMaxScore(String(p.maxScore));
    }
    if (p.scopeCatalogId != null) setScope(String(p.scopeCatalogId));
    p.clear();
  }, []);

  const groups = taxonomy.data?.groups ?? [];
  const group = groups.find((g) => g.id === grp);
  const label = grp && value ? name.label(grp, value) : value;
  const userCatalogs = (catalogs.data ?? []).filter((c) => !c.catalog.system);
  const catalogId = scope === LIBRARY ? null : Number(scope);
  const score = limit ? Number(maxScore) : null;
  const scoreValid = !limit || (maxScore.trim() !== "" && score! >= 0 && score! <= 1);
  const ready = grp !== "" && value !== "" && scoreValid;

  const current = `${ready}|${grp}|${value}|${catalogId}|${score}`;
  const debounced = useDebounced(current, 250);
  const stable = debounced === current;
  const preview = useQuery({
    queryKey: ["ai-reject-preview", debounced],
    enabled: ready && stable,
    queryFn: async () => commands.aiRejectPreview(grp, value, score, await scopeTarget(catalogId)),
  });
  const count = ready && stable ? preview.data : undefined;

  const submit = async () => {
    try {
      const target = await scopeTarget(catalogId);
      await start(grp, value, score, target, t("aiCleanup.done", { label, count: count ?? 0 }));
    } catch (error) {
      notifyError(error);
    }
  };

  return (
    <section className="ai-section" aria-labelledby="ai-section-title">
      <h2 id="ai-section-title">{t("ai.sections.cleanup")}</h2>
      <p className="muted">{t("aiCleanup.intro")}</p>
      <div className="ai-cleanup-form">
        <FormField label={t("aiCleanup.group")}>
          <select
            className="field"
            value={grp}
            onChange={(e) => {
              setGrp(e.target.value);
              setValue("");
            }}
          >
            <option value="">{t("aiCleanup.pickGroup")}</option>
            {groups.map((g) => (
              <option key={g.id} value={g.id}>
                {name.group(g.id)}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label={t("aiCleanup.value")}>
          <select
            className="field"
            value={value}
            disabled={!group}
            onChange={(e) => setValue(e.target.value)}
          >
            <option value="">{t("aiCleanup.pickValue")}</option>
            {(group?.categories ?? []).map((c) => (
              <option key={c.id} value={c.id}>
                {name.label(group!.id, c.id)}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label={t("aiCleanup.scope")}>
          <select className="field" value={scope} onChange={(e) => setScope(e.target.value)}>
            <option value={LIBRARY}>{t("aiCleanup.scopeLibrary")}</option>
            {userCatalogs.map((c) => (
              <option key={c.catalog.id} value={String(c.catalog.id)}>
                {t("aiCleanup.scopeCatalog", { name: c.catalog.name })}
              </option>
            ))}
          </select>
        </FormField>
        <div className="ai-cleanup-score">
          <label>
            <input type="checkbox" checked={limit} onChange={(e) => setLimit(e.target.checked)} />{" "}
            {t("aiCleanup.onlyBelow")}
          </label>
          {limit && (
            <FormField
              label={t("aiCleanup.maxScore")}
              hint={t("aiCleanup.maxScoreHint")}
              error={scoreValid ? undefined : t("aiCleanup.maxScoreHint")}
            >
              <input
                className="field"
                type="number"
                min={0}
                max={1}
                step={0.05}
                value={maxScore}
                onChange={(e) => setMaxScore(e.target.value)}
              />
            </FormField>
          )}
        </div>
      </div>
      <p className="ai-cleanup-preview" role="status" aria-live="polite">
        {!ready
          ? ""
          : preview.isError
            ? t("aiCleanup.previewError")
            : count === undefined
              ? t("aiCleanup.previewing")
              : count === 0
                ? t("aiCleanup.previewNone", { label })
                : t("aiCleanup.preview", { count, label })}
      </p>
      <Button variant="primary" disabled={!count || busy} onClick={() => setConfirming(true)}>
        {t("aiCleanup.submit")}
      </Button>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t("aiCleanup.confirmTitle", { label })}
        message={t("aiCleanup.confirmMessage", { label, count: count ?? 0 })}
        confirmLabel={t("aiCleanup.confirmLabel")}
        danger
        onConfirm={submit}
      />
    </section>
  );
}
