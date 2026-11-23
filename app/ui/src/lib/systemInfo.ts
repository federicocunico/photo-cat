// Informazioni sull'app installata (app-about): testo per "Copia informazioni" e righe della
// finestra Informazioni. Il percorso dei dati non va negli appunti salvo richiesta esplicita.
import type { TFunction } from "i18next";
import { commands, isTauri, type AppInfo } from "./ipc";
import { fmtDate } from "./format";
import { notify, notifyError } from "../store/toasts";

/** "a1b2c3d · 23 nov 2026 · release", oppure "commit non disponibile · …". */
export function buildText(info: AppInfo, t: TFunction): string {
  const date = info.buildDate ? fmtDate(info.buildDate) : null;
  return [info.commit ?? t("about.noCommit"), date, info.profile].filter(Boolean).join(" · ");
}

/** Righe "etichetta: valore" da incollare in una segnalazione (senza il percorso dei dati). */
export function systemInfoLines(info: AppInfo, t: TFunction): string[] {
  return [
    `${t("about.app")}: photo-cat ${info.version}`,
    `${t("about.build")}: ${buildText(info, t)}`,
    `${t("about.system")}: ${info.os} ${info.arch}`,
    `${t("about.webview")}: ${info.webview ?? t("about.unknown")}`,
    `${t("about.accelerator")}: ${info.accelerator}`,
  ];
}

export async function copySystemInfo(t: TFunction): Promise<void> {
  if (!isTauri()) return;
  try {
    const info = await commands.appInfo();
    await navigator.clipboard.writeText(systemInfoLines(info, t).join("\n"));
    notify({ tone: "ok", message: t("menu.infoCopied") });
  } catch (error) {
    notifyError(error);
  }
}
