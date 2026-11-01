// Sezione "Applicazioni esterne": app per "Apri con", azione del doppio clic e
// cartelle da ignorare nella deduzione del luogo. Si salva con il pulsante
// generale delle impostazioni (il backend verifica che gli eseguibili esistano).
// Le righe vuote di argomenti e cartelle si tolgono al salvataggio (`cleanApps`).
import { useTranslation } from "react-i18next";
import type { ExternalApp, Settings } from "../../lib/ipc";

const newId = (name: string, taken: string[]) => {
  const base =
    name
      .toLowerCase()
      .normalize("NFD")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || "app";
  let id = base;
  for (let i = 2; taken.includes(id) || id === "system" || id === "none"; i++) id = `${base}-${i}`;
  return id;
};

export function ExternalAppsSection({
  cfg,
  setCfg,
}: {
  cfg: Settings;
  setCfg: (c: Settings) => void;
}) {
  const { t } = useTranslation();
  const apps: ExternalApp[] = cfg.external_apps ?? [];
  const setApps = (next: ExternalApp[]) => {
    const ids = next.map((a) => a.id);
    // Un doppio clic verso un'app rimossa torna all'app predefinita.
    const dc = cfg.ui.double_click ?? "system";
    setCfg({
      ...cfg,
      external_apps: next,
      ui: {
        ...cfg.ui,
        double_click: dc === "system" || dc === "none" || ids.includes(dc) ? dc : "system",
      },
    });
  };
  const edit = (i: number, patch: Partial<ExternalApp>) =>
    setApps(apps.map((a, j) => (j === i ? { ...a, ...patch } : a)));
  const browse = async (i: number) => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({ multiple: false, directory: false });
    if (typeof picked === "string") {
      const name =
        apps[i].name ||
        picked
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.(exe|app)$/i, "") ||
        "";
      edit(i, { path: picked, name });
    }
  };
  return (
    <section aria-labelledby="external-apps-title" style={{ margin: "24px 0" }}>
      <h2 id="external-apps-title">{t("externalApps.title")}</h2>
      <p className="muted">{t("externalApps.hint")}</p>
      <div className="feature-card" style={{ alignItems: "stretch", gap: 10 }}>
        {apps.map((a, i) => (
          <fieldset
            key={i}
            style={{
              border: "1px solid var(--pc-border)",
              display: "flex",
              flexWrap: "wrap",
              gap: 8,
            }}
          >
            <legend>{a.name || t("externalApps.name")}</legend>
            <label>
              {t("externalApps.name")}{" "}
              <input
                className="field"
                value={a.name ?? ""}
                onChange={(e) => edit(i, { name: e.target.value })}
              />
            </label>
            <label style={{ flex: 1, minWidth: 260 }}>
              {t("externalApps.path")}{" "}
              <input
                className="field mono"
                style={{ width: "100%" }}
                value={a.path ?? ""}
                onChange={(e) => edit(i, { path: e.target.value })}
              />
            </label>
            <button className="btn" type="button" onClick={() => void browse(i)}>
              {t("externalApps.browse")}
            </button>
            <label style={{ flexBasis: "100%" }}>
              {t("externalApps.args")}
              <textarea
                className="field mono"
                rows={2}
                style={{ width: "100%" }}
                placeholder="{file}"
                value={(a.args ?? []).join("\n")}
                onChange={(e) => edit(i, { args: e.target.value.split("\n") })}
              />
            </label>
            <button
              className="btn btn-ghost"
              type="button"
              aria-label={`${t("externalApps.remove")} ${a.name ?? ""}`}
              onClick={() => setApps(apps.filter((_, j) => j !== i))}
            >
              {t("externalApps.remove")}
            </button>
          </fieldset>
        ))}
        <button
          className="btn"
          type="button"
          onClick={() =>
            setApps([
              ...apps,
              {
                id: newId(
                  `app-${apps.length + 1}`,
                  apps.map((a) => a.id ?? ""),
                ),
                name: "",
                path: "",
                args: ["{file}"],
              },
            ])
          }
        >
          {t("externalApps.add")}
        </button>
        <label>
          {t("externalApps.doubleClick")}{" "}
          <select
            className="field"
            value={cfg.ui.double_click ?? "system"}
            onChange={(e) => setCfg({ ...cfg, ui: { ...cfg.ui, double_click: e.target.value } })}
          >
            <option value="system">{t("externalApps.system")}</option>
            {apps.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name || a.id}
              </option>
            ))}
            <option value="none">{t("externalApps.none")}</option>
          </select>
        </label>
        <label>
          {t("externalApps.ignoredFolders")}
          <textarea
            className="field"
            rows={3}
            style={{ width: "100%" }}
            value={(cfg.ai.ignored_folder_names ?? []).join("\n")}
            onChange={(e) =>
              setCfg({
                ...cfg,
                ai: { ...cfg.ai, ignored_folder_names: e.target.value.split("\n") },
              })
            }
          />
        </label>
        <p className="muted" style={{ margin: 0 }}>
          {t("externalApps.ignoredHint")}
        </p>
      </div>
    </section>
  );
}
