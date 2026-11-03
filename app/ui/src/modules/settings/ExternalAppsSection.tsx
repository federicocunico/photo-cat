// Impostazioni › Applicazioni esterne: app per "Apri con" e per il doppio clic. Ogni
// modifica si salva dopo una breve pausa; il motore verifica che gli eseguibili
// esistano e l'eventuale errore compare accanto all'elenco. Le righe incomplete (senza
// nome o eseguibile) restano solo qui finché non sono complete.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui";
import type { ExternalApp } from "../../lib/ipc";
import { Setting } from "./settingsState";
import { useSettings } from "./settingsContext";

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

const complete = (a: ExternalApp) => Boolean(a.name?.trim() && a.path?.trim());

export function ExternalAppsSection() {
  const { t } = useTranslation();
  const { saved, autosave } = useSettings();
  const [apps, setLocal] = useState<ExternalApp[]>(() => saved.external_apps ?? []);
  const setApps = (next: ExternalApp[]) => {
    setLocal(next);
    const ready = next.filter(complete);
    const ids = ready.map((a) => a.id);
    autosave("externalApps", (c) => {
      // Un doppio clic verso un'app rimossa torna all'app predefinita.
      const dc = c.ui.double_click ?? "system";
      return {
        ...c,
        external_apps: ready,
        ui: {
          ...c.ui,
          double_click:
            ["system", "viewer", "none"].includes(dc) || ids.includes(dc) ? dc : "system",
        },
      };
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
    <Setting id="externalApps" label={t("externalApps.title")} hint={t("externalApps.hint")}>
      <div className="settings-apps">
        {apps.map((a, i) => (
          <fieldset key={i} className="settings-app">
            <legend>{a.name || t("externalApps.name")}</legend>
            <label>
              <span>{t("externalApps.name")}</span>
              <input
                className="field"
                value={a.name ?? ""}
                onChange={(e) => edit(i, { name: e.target.value })}
              />
            </label>
            <label className="settings-app-path">
              <span>{t("externalApps.path")}</span>
              <input
                className="field mono"
                value={a.path ?? ""}
                spellCheck={false}
                onChange={(e) => edit(i, { path: e.target.value })}
              />
            </label>
            <Button onClick={async () => await browse(i)}>{t("externalApps.browse")}</Button>
            <label className="settings-app-args">
              <span>{t("externalApps.args")}</span>
              <textarea
                className="field mono"
                rows={2}
                placeholder="{file}"
                value={(a.args ?? []).join("\n")}
                onChange={(e) => edit(i, { args: e.target.value.split("\n") })}
              />
            </label>
            <Button
              variant="ghost"
              aria-label={`${t("externalApps.remove")} ${a.name ?? ""}`}
              onClick={() => setApps(apps.filter((_, j) => j !== i))}
            >
              {t("externalApps.remove")}
            </Button>
          </fieldset>
        ))}
        <Button
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
        </Button>
      </div>
    </Setting>
  );
}
