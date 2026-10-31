import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Settings,
  type Taxonomy,
  type ExecutionProvider,
  type ResourceProfile,
  type LogLevel,
} from "../../lib/ipc";
import { useLayout } from "../../store/layout";
export function SettingsPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const config = useQuery({
    queryKey: ["config"],
    queryFn: commands.getConfig,
    enabled: isTauri(),
  });
  const system = useQuery({
    queryKey: ["system"],
    queryFn: commands.systemInfo,
    enabled: isTauri(),
  });
  const info = useQuery({ queryKey: ["app-info"], queryFn: commands.appInfo, enabled: isTauri() });
  const taxonomy = useQuery({
    queryKey: ["taxonomy"],
    queryFn: commands.getTaxonomy,
    enabled: isTauri(),
  });
  const faces = useQuery({
    queryKey: ["face-count"],
    queryFn: commands.faceCount,
    enabled: isTauri(),
  });
  const [cfg, setCfg] = useState<Settings | null>(null);
  const [tax, setTax] = useState<Taxonomy | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (config.data) setCfg(config.data as Settings);
  }, [config.data]);
  useEffect(() => {
    if (taxonomy.data) setTax(taxonomy.data);
  }, [taxonomy.data]);
  const run = async (fn: () => Promise<void>) => {
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
  if (!cfg) return <p style={{ padding: 24 }}>{t("settings.loading")}</p>;
  const save = async () => {
    await commands.saveSettings(cfg);
    useLayout.getState().setTheme(cfg.ui.theme);
    useLayout.getState().setLanguage(cfg.ui.language);
    await qc.invalidateQueries({ queryKey: ["config"] });
    await qc.invalidateQueries({ queryKey: ["system"] });
    await qc.invalidateQueries({ queryKey: ["taxonomy"] });
    setNotice(t("settings.saved"));
  };
  return (
    <section style={{ padding: 24, overflow: "auto", flex: 1 }}>
      <h1>{t("settings.title")}</h1>
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      <div className="feature-card" style={{ alignItems: "stretch" }}>
        <label>
          {t("settings.profile")}{" "}
          <select
            className="field"
            value={cfg.resources.profile}
            onChange={(e) =>
              setCfg({
                ...cfg,
                resources: { ...cfg.resources, profile: e.target.value as ResourceProfile },
              })
            }
          >
            {["light", "balanced", "performance"].map((p) => (
              <option key={p} value={p}>
                {t(`models.profiles.${p}`, { defaultValue: p })}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("settings.ram")}{" "}
          <input
            className="field"
            type="number"
            min={512}
            step={256}
            value={cfg.resources.ram_limit_mb ?? ""}
            onChange={(e) =>
              setCfg({
                ...cfg,
                resources: {
                  ...cfg.resources,
                  ram_limit_mb: e.target.value ? Number(e.target.value) : null,
                },
              })
            }
          />
        </label>
        <label>
          {t("settings.provider")}{" "}
          <select
            className="field"
            value={cfg.ai.execution_provider}
            onChange={(e) =>
              setCfg({
                ...cfg,
                ai: { ...cfg.ai, execution_provider: e.target.value as ExecutionProvider },
              })
            }
          >
            {["auto", ...(system.data?.providers ?? ["cpu"])].map((p) => (
              <option key={p} value={p}>
                {p.toUpperCase()}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("settings.threshold")}{" "}
          <input
            className="field"
            type="number"
            min={0}
            max={1}
            step={0.01}
            value={cfg.ai.confidence_threshold ?? 0}
            onChange={(e) =>
              setCfg({ ...cfg, ai: { ...cfg.ai, confidence_threshold: Number(e.target.value) } })
            }
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={cfg.ai.auto_download_models}
            onChange={(e) =>
              setCfg({ ...cfg, ai: { ...cfg.ai, auto_download_models: e.target.checked } })
            }
          />
          {t("settings.autoModels")}
        </label>
        <label>
          <input
            type="checkbox"
            checked={cfg.ai.faces_enabled}
            onChange={(e) => setCfg({ ...cfg, ai: { ...cfg.ai, faces_enabled: e.target.checked } })}
          />
          {t("settings.faces")}
        </label>
        <label>
          {t("settings.faceSize")}{" "}
          <input
            className="field"
            type="number"
            min={8}
            value={cfg.ai.face_min_size_px}
            onChange={(e) =>
              setCfg({ ...cfg, ai: { ...cfg.ai, face_min_size_px: Number(e.target.value) } })
            }
          />
        </label>
        <button
          className="btn"
          disabled={busy}
          onClick={() => {
            if (window.confirm(t("settings.purgeConfirm", { count: faces.data ?? 0 })))
              void run(async () => {
                await commands.purgeFaces();
                await qc.invalidateQueries({ queryKey: ["config"] });
                await qc.invalidateQueries({ queryKey: ["face-count"] });
                setNotice(t("settings.purged"));
              });
          }}
        >
          {t("settings.purge")}
        </button>
        <label>
          {t("settings.log")}{" "}
          <select
            className="field"
            value={cfg.logging.level}
            onChange={(e) =>
              setCfg({ ...cfg, logging: { ...cfg.logging, level: e.target.value as LogLevel } })
            }
          >
            {["error", "warn", "info", "debug", "trace"].map((l) => (
              <option key={l}>{l}</option>
            ))}
          </select>
        </label>
        <label>
          {t("settings.theme")}{" "}
          <select
            className="field"
            value={cfg.ui.theme}
            onChange={(e) =>
              setCfg({ ...cfg, ui: { ...cfg.ui, theme: e.target.value as "dark" | "light" } })
            }
          >
            <option value="dark">{t("settings.dark")}</option>
            <option value="light">{t("settings.light")}</option>
          </select>
        </label>
        <label>
          {t("settings.language")}{" "}
          <select
            className="field"
            value={cfg.ui.language}
            onChange={(e) =>
              setCfg({ ...cfg, ui: { ...cfg.ui, language: e.target.value as "it" | "en" } })
            }
          >
            <option value="it">Italiano</option>
            <option value="en">English</option>
          </select>
        </label>
        <p className="mono">
          {t("settings.currentPath")}: {info.data?.dataDir}
        </p>
        <label>
          {t("settings.path")}{" "}
          <input
            className="field"
            value={cfg.data_dir ?? ""}
            onChange={(e) => setCfg({ ...cfg, data_dir: e.target.value || null })}
          />
        </label>
        <p className="muted">{t("settings.pathHint")}</p>
        <button className="btn" disabled={busy} onClick={() => void run(save)}>
          {t("people.save")}
        </button>
      </div>
      {tax && (
        <>
          <h2>{t("settings.taxonomy")}</h2>
          <p className="muted">{t("settings.taxonomyHint")}</p>
          {tax.groups.map((g, gi) => (
            <fieldset key={g.id} style={{ margin: "16px 0", border: "1px solid var(--pc-border)" }}>
              <legend>{g.label}</legend>
              <label>
                {t("settings.groupThreshold")}{" "}
                <input
                  className="field"
                  type="number"
                  min={0}
                  max={1}
                  step={0.01}
                  value={g.threshold ?? 0}
                  onChange={(e) =>
                    setTax({
                      ...tax,
                      groups: tax.groups.map((v, i) =>
                        i === gi ? { ...v, threshold: Number(e.target.value) } : v,
                      ),
                    })
                  }
                />
              </label>
              {g.categories.map((c, ci) => (
                <div className="feature-actions" key={c.id}>
                  <input
                    type="checkbox"
                    aria-label={`${t("settings.enabled")} ${c.label}`}
                    checked={c.enabled}
                    onChange={(e) =>
                      setTax({
                        ...tax,
                        groups: tax.groups.map((v, i) =>
                          i === gi
                            ? {
                                ...v,
                                categories: v.categories.map((a, j) =>
                                  j === ci ? { ...a, enabled: e.target.checked } : a,
                                ),
                              }
                            : v,
                        ),
                      })
                    }
                  />
                  <input
                    className="field"
                    aria-label={`${t("people.name")} ${c.id}`}
                    value={c.label}
                    onChange={(e) =>
                      setTax({
                        ...tax,
                        groups: tax.groups.map((v, i) =>
                          i === gi
                            ? {
                                ...v,
                                categories: v.categories.map((a, j) =>
                                  j === ci ? { ...a, label: e.target.value } : a,
                                ),
                              }
                            : v,
                        ),
                      })
                    }
                  />
                  <textarea
                    className="field"
                    aria-label={`${t("settings.prompts")} ${c.id}`}
                    value={c.prompts.join("\n")}
                    onChange={(e) =>
                      setTax({
                        ...tax,
                        groups: tax.groups.map((v, i) =>
                          i === gi
                            ? {
                                ...v,
                                categories: v.categories.map((a, j) =>
                                  j === ci ? { ...a, prompts: e.target.value.split("\n") } : a,
                                ),
                              }
                            : v,
                        ),
                      })
                    }
                  />
                </div>
              ))}
              <button
                className="btn"
                onClick={() =>
                  setTax({
                    ...tax,
                    groups: tax.groups.map((v, i) =>
                      i === gi
                        ? {
                            ...v,
                            categories: [
                              ...v.categories,
                              {
                                id: `custom_${Date.now()}`,
                                label: t("settings.newCategory"),
                                enabled: true,
                                prompts: ["a photo of a new subject"],
                              },
                            ],
                          }
                        : v,
                    ),
                  })
                }
              >
                {t("settings.addCategory")}
              </button>
            </fieldset>
          ))}
          <button
            className="btn"
            disabled={busy}
            onClick={() =>
              void run(async () => {
                await commands.saveTaxonomy(tax, taxonomy.data!.version);
                await qc.invalidateQueries({ queryKey: ["taxonomy"] });
                setNotice(t("settings.saved"));
              })
            }
          >
            {t("settings.saveTaxonomy")}
          </button>
        </>
      )}
    </section>
  );
}
