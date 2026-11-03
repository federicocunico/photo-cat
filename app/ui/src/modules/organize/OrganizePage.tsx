// Modulo Esporta: "Esporta dati", "Organizza in cartelle" e "Sequenza" (design, decisione 3).
import { useEffect, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Method,
  type Organization,
  type Preflight,
  type Preview,
  type Scope,
} from "../../lib/ipc";
import { selCount, selTarget, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { useSequence } from "../../store/sequence";
import { notify } from "../../store/toasts";
import { fmtBytes, fmtInt } from "../../lib/format";
import {
  Badge,
  Button,
  Card,
  ConfirmDialog,
  Disclosure,
  FolderPicker,
  FormField,
  InlineAlert,
  PageHeader,
  type Tone,
} from "../../components/ui";
import { Icon } from "../../components/icons";
import "./organize.css";

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
const METHODS: Method[] = ["copy", "hardlink", "symlink"];
type ScopeName = "selection" | "filter" | "library" | "catalog";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

/** Avviso di avvio con il comando per seguire l'attività in Attività. */
function useQueued() {
  const { t } = useTranslation();
  return (message: string, job: number) =>
    notify({
      tone: "ok",
      message,
      action: {
        label: t("organize.follow"),
        run: () => useLayout.getState().openModule({ module: "jobs", target: job }),
      },
    });
}

/** Un passo numerato con il suo stato in parole (e un'icona, non solo il colore). */
function Step({
  title,
  state,
  tone,
  children,
}: {
  title: string;
  state: string;
  tone: Tone | "neutral";
  children?: ReactNode;
}) {
  return (
    <li className="export-step">
      <div className="export-step-head">
        <strong>{title}</strong>
        <Badge tone={tone}>
          {tone === "ok" && <Icon.Check size={12} />}
          {tone === "danger" && <Icon.Alert size={12} />}
          {state}
        </Badge>
      </div>
      {children}
    </li>
  );
}

export function OrganizePage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const queued = useQueued();
  const { filter, selection } = useLibrary();
  const selectedCount = selCount(selection);
  // Con una selezione si esporta quella; altrimenti ciò che la Libreria mostra.
  const [scopeName, setScope] = useState<ScopeName>(() =>
    selectedCount > 0 ? "selection" : "filter",
  );
  const [catalog, setCatalog] = useState("");
  const [destination, setDestination] = useState("");
  const [exportPath, setExportPath] = useState("");
  const [format, setFormat] = useState<"csv" | "json">("csv");
  const [separator, setSeparator] = useState(";");
  const [bom, setBom] = useState(true);
  const [fields, setFields] = useState<string[]>([]);
  const [method, setMethod] = useState<Method>("copy");
  const [levels, setLevels] = useState(["place", "year"]);
  const [exportError, setExportError] = useState("");
  const [organizeError, setOrganizeError] = useState("");
  const [preview, setPreview] = useState<{ key: string; data: Preview } | null>(null);
  const [check, setCheck] = useState<{ key: string; data: Preflight } | null>(null);
  const [confirmCreate, setConfirmCreate] = useState(false);
  const [undoing, setUndoing] = useState<Organization | null>(null);
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
          : selection.mode === "ids"
            ? { scope: "selection", ids: [...selection.ids] }
            : { scope: "target", target: selTarget(selection) };
  const options = { scope, destination, levels, method };
  const key = JSON.stringify(options);
  const scopeReason =
    scopeName === "selection" && selectedCount === 0
      ? t("organize.noSelection")
      : scopeName === "catalog" && !catalog
        ? t("organize.chooseCatalog")
        : undefined;

  const pickFile = async () => {
    if (!isTauri()) return null;
    const dialog = await import("@tauri-apps/plugin-dialog");
    const path = await dialog.save({
      defaultPath: `catalog.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (typeof path === "string") setExportPath(path);
  };
  const pickFolder = async () => {
    if (!isTauri()) return null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({ directory: true, multiple: false });
    return typeof path === "string" ? path : null;
  };

  // Esporta dati.
  const exportReason =
    scopeReason ??
    (!exportPath
      ? t("organize.needExportPath")
      : !fields.length
        ? t("organize.needFields")
        : format === "csv" && !separator
          ? t("organize.needSeparator")
          : undefined);
  const startExport = async () => {
    setExportError("");
    try {
      const job = await commands.startExport({
        scope,
        destination: exportPath,
        format,
        separator,
        bom,
        fields,
      });
      queued(t("organize.queuedExport"), job);
    } catch (e) {
      setExportError(errorText(e));
    }
  };

  // Organizza in cartelle: anteprima → verifica → crea.
  const optionsReason = scopeReason ?? (!destination ? t("organize.needDestination") : undefined);
  const previewOk = preview?.key === key;
  const checkOk = check?.key === key;
  const methodOk = checkOk && check.data.available;
  const createReason =
    optionsReason ??
    (!previewOk || !checkOk
      ? t("organize.needSteps")
      : !methodOk
        ? t("organize.needMethod")
        : undefined);
  const step = async (fn: () => Promise<void>) => {
    setOrganizeError("");
    try {
      await fn();
    } catch (e) {
      setOrganizeError(errorText(e));
    }
  };
  const stepState = (done: boolean, has: boolean, failed = false) =>
    done
      ? failed
        ? { state: t("organize.failed"), tone: "danger" as const }
        : { state: t("organize.done"), tone: "ok" as const }
      : has
        ? { state: t("organize.stale"), tone: "warn" as const }
        : { state: t("organize.todo"), tone: "neutral" as const };
  const s1 = stepState(previewOk, preview !== null);
  const s2 = stepState(checkOk, check !== null, checkOk && !check.data.available);
  const s3 = createReason
    ? { state: t("organize.createWaiting"), tone: "neutral" as const }
    : { state: t("organize.createReady"), tone: "info" as const };

  const openSequenceExport = () => {
    useSequence.getState().setView("export");
    useLayout.getState().openModule({ module: "sequence" });
  };

  return (
    <div className="export-page">
      <PageHeader title={t("modules.organize")} description={t("organize.subtitle")} />
      <div className="export-body">
        <Card title={t("organize.scopeTitle")}>
          <div className="export-row">
            <FormField
              label={t("organize.scope")}
              hint={scopeName === "filter" ? t("organize.scopeFilterHint") : undefined}
            >
              <select
                className="field"
                value={scopeName}
                onChange={(e) => setScope(e.target.value as ScopeName)}
              >
                <option value="selection">
                  {t("organize.scopeSelection", { count: selectedCount })}
                </option>
                <option value="filter">{t("organize.scopes.filter")}</option>
                <option value="library">{t("organize.scopes.library")}</option>
                <option value="catalog">{t("organize.scopes.catalog")}</option>
              </select>
            </FormField>
            {scopeName === "catalog" && (
              <FormField label={t("organize.scopes.catalog")}>
                <select
                  className="field"
                  value={catalog}
                  onChange={(e) => setCatalog(e.target.value)}
                >
                  <option value="">{t("organize.catalogNone")}</option>
                  {catalogs.data?.map(({ catalog: c }) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </FormField>
            )}
          </div>
          {scopeName === "selection" && selectedCount === 0 && (
            <InlineAlert tone="warn">{t("organize.noSelection")}</InlineAlert>
          )}
        </Card>

        <Card title={t("organize.export")}>
          <p className="muted export-hint">{t("organize.exportHint")}</p>
          <div className="export-row">
            <FormField label={t("organize.format")}>
              <select
                className="field"
                value={format}
                onChange={(e) => setFormat(e.target.value as "csv" | "json")}
              >
                <option value="csv">CSV</option>
                <option value="json">JSON</option>
              </select>
            </FormField>
            <div className="export-file">
              <FormField label={t("organize.exportPath")}>
                <input
                  className="field mono"
                  value={exportPath}
                  spellCheck={false}
                  onChange={(e) => setExportPath(e.target.value)}
                />
              </FormField>
              <Button onClick={pickFile}>{t("organize.browse")}</Button>
            </div>
          </div>
          <fieldset className="export-fields">
            <legend>{t("organize.fields")}</legend>
            <div>
              {allFields.data?.map((f) => (
                <label key={f} className="export-check">
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
          {format === "csv" && (
            <Disclosure title={t("organize.advanced")} defaultOpen={false}>
              <div className="export-row">
                <FormField label={t("organize.separator")}>
                  <input
                    className="field export-separator"
                    value={separator}
                    maxLength={1}
                    onChange={(e) => setSeparator(e.target.value)}
                  />
                </FormField>
                <label className="export-check">
                  <input type="checkbox" checked={bom} onChange={(e) => setBom(e.target.checked)} />
                  {t("organize.bom")}
                </label>
              </div>
            </Disclosure>
          )}
          {exportError && <InlineAlert tone="danger">{exportError}</InlineAlert>}
          <div className="export-actions">
            <Button
              variant="primary"
              icon={<Icon.Export size={16} />}
              disabledReason={exportReason}
              onClick={startExport}
            >
              {t("organize.exportStart")}
            </Button>
          </div>
        </Card>

        <Card title={t("organize.folders")}>
          <p className="muted export-hint">{t("organize.foldersHint")}</p>
          <FolderPicker
            label={t("organize.destination")}
            value={destination}
            onChange={setDestination}
            pick={pickFolder}
          />
          <FormField label={t("organize.method")} hint={t(`organize.methodHints.${method}`)}>
            <select
              className="field"
              value={method}
              onChange={(e) => setMethod(e.target.value as Method)}
            >
              {METHODS.map((m) => (
                <option value={m} key={m}>
                  {t(`organize.methods.${m}`)}
                </option>
              ))}
            </select>
          </FormField>
          <fieldset className="export-levels">
            <legend>{t("organize.levelsTitle")}</legend>
            <ol>
              {levels.map((level, i) => (
                <li key={i}>
                  <select
                    className="field"
                    aria-label={`${t("organize.level")} ${i + 1}`}
                    value={level}
                    onChange={(e) =>
                      setLevels((s) => s.map((l, j) => (i === j ? e.target.value : l)))
                    }
                  >
                    {LEVELS.map((l) => (
                      <option key={l} value={l}>
                        {t(`organize.levels.${l}`)}
                      </option>
                    ))}
                  </select>
                  <Button
                    size="sm"
                    variant="ghost"
                    icon={<Icon.ArrowUp size={14} />}
                    aria-label={t("organize.levelUpOf", { n: i + 1 })}
                    disabled={i === 0}
                    onClick={() =>
                      setLevels((s) => {
                        const v = [...s];
                        [v[i - 1], v[i]] = [v[i], v[i - 1]];
                        return v;
                      })
                    }
                  >
                    {t("organize.levelUp")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    icon={<Icon.Close size={14} />}
                    aria-label={t("organize.levelRemoveOf", { n: i + 1 })}
                    disabled={levels.length === 1}
                    onClick={() => setLevels((s) => s.filter((_, j) => j !== i))}
                  >
                    {t("organize.levelRemove")}
                  </Button>
                </li>
              ))}
            </ol>
            <div>
              <Button
                size="sm"
                icon={<Icon.Plus size={14} />}
                disabled={levels.length >= 6}
                onClick={() => setLevels((s) => [...s, "year"])}
              >
                {t("organize.addLevel")}
              </Button>
            </div>
          </fieldset>

          <ol className="export-steps" aria-label={t("organize.steps")}>
            <Step title={t("organize.stepPreview")} {...s1}>
              <div>
                <Button
                  disabledReason={optionsReason}
                  onClick={() =>
                    step(async () =>
                      setPreview({ key, data: await commands.organizePreview(options) }),
                    )
                  }
                >
                  {t("organize.preview")}
                </Button>
              </div>
              {previewOk && (
                <>
                  <p>
                    {t("organize.summary", {
                      count: preview.data.files,
                      conflicts: preview.data.conflicts,
                      space: fmtBytes(method === "copy" ? preview.data.bytes : 0),
                    })}
                  </p>
                  <ul className="mono export-preview">
                    {Object.entries(preview.data.folders).map(([p, n]) => (
                      <li key={p}>{t("organize.folderCount", { path: p, n: fmtInt(n) })}</li>
                    ))}
                  </ul>
                </>
              )}
            </Step>
            <Step title={t("organize.stepVerify")} {...s2}>
              <div>
                <Button
                  disabledReason={optionsReason}
                  onClick={() =>
                    step(async () =>
                      setCheck({ key, data: await commands.organizePreflight(options) }),
                    )
                  }
                >
                  {t("organize.verify")}
                </Button>
              </div>
              {checkOk && (
                <p role="status">
                  {check.data.available ? t("organize.available") : t("organize.unavailable")}{" "}
                  {check.data.reason && t(`organize.reasons.${check.data.reason}`)}{" "}
                  {!check.data.available && t("organize.alternative")}{" "}
                  {!check.data.available && (
                    <Button size="sm" onClick={() => setMethod(check.data.alternative)}>
                      {t(`organize.methods.${check.data.alternative}`)}
                    </Button>
                  )}
                </p>
              )}
            </Step>
            <Step title={t("organize.stepCreate")} {...s3}>
              <div>
                <Button
                  variant="primary"
                  icon={<Icon.Organize size={16} />}
                  disabledReason={createReason}
                  onClick={() => setConfirmCreate(true)}
                >
                  {t("organize.start")}
                </Button>
              </div>
            </Step>
          </ol>
          {organizeError && <InlineAlert tone="danger">{organizeError}</InlineAlert>}
          <ConfirmDialog
            open={confirmCreate}
            onOpenChange={setConfirmCreate}
            title={t("organize.createTitle", {
              count: preview?.data.files ?? 0,
              path: destination,
            })}
            message={t(`organize.createBody.${method}`)}
            confirmLabel={t("organize.createConfirm", { count: preview?.data.files ?? 0 })}
            count={preview?.data.files ?? 0}
            files="affected"
            danger={false}
            onConfirm={async () => {
              const job = await commands.startOrganize(options);
              queued(t("organize.queuedOrganize"), job);
              await qc.invalidateQueries({ queryKey: ["organizations"] });
            }}
          />
        </Card>

        <Card title={t("organize.sequence")}>
          <p className="muted export-hint">{t("organize.sequenceHint")}</p>
          <div>
            <Button icon={<Icon.Sequence size={16} />} onClick={openSequenceExport}>
              {t("organize.sequenceOpen")}
            </Button>
          </div>
        </Card>

        <Card title={t("organize.history")}>
          <History items={history.data ?? []} onUndo={setUndoing} />
          <ConfirmDialog
            open={undoing !== null}
            onOpenChange={(o) => !o && setUndoing(null)}
            title={t("organize.undoTitle", { path: undoing?.dest ?? "" })}
            message={t("organize.undoConfirm")}
            confirmLabel={t("organize.undoConfirmLabel", { count: undoing?.entries ?? 0 })}
            count={undoing?.entries}
            files="affected"
            onConfirm={async () => {
              if (!undoing) return;
              const [removed, kept] = await commands.undoOrganization(undoing.id, true);
              notify({ tone: "ok", message: t("organize.undone", { removed, kept }) });
              await qc.invalidateQueries({ queryKey: ["organizations"] });
            }}
          />
        </Card>
      </div>
    </div>
  );
}

/** Storico leggibile: dove, con che metodo, quanti elementi e in che stato. */
function History({ items, onUndo }: { items: Organization[]; onUndo: (o: Organization) => void }) {
  const { t } = useTranslation();
  const visible = items.filter((o) => o.state !== "undone");
  if (!visible.length) return <p className="muted export-hint">{t("organize.historyEmpty")}</p>;
  return (
    <ul className="export-history">
      {visible.map((o) => (
        <li key={o.id}>
          <div>
            <strong className="mono export-path">
              {t("organize.historyItem", { path: o.dest })}
            </strong>
            <span className="muted">
              {t("organize.historyMeta", {
                count: o.entries,
                method: t(`organize.methods.${o.method}`, { defaultValue: o.method }),
                state: t(`organize.states.${o.state}`, { defaultValue: o.state }),
              })}
            </span>
          </div>
          <Button
            size="sm"
            icon={<Icon.Delete size={14} />}
            disabledReason={o.state === "running" ? t("organize.undoRunning") : undefined}
            onClick={() => onUndo(o)}
          >
            {t("organize.undo")}
          </Button>
        </li>
      ))}
    </ul>
  );
}
