// "Apri con": apre i file con l'app predefinita, un'applicazione esterna o il
// file manager; oltre il limite del backend chiede conferma.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type OpenTarget } from "../../lib/ipc";

export function useOpenWith() {
  const { t } = useTranslation();
  const [error, setError] = useState("");
  const open = async (ids: number[], target: OpenTarget) => {
    setError("");
    try {
      const r = await commands.openWith(ids, target, false);
      if (
        r.needsConfirmation &&
        window.confirm(t("externalApps.confirmMany", { count: ids.length }))
      )
        await commands.openWith(ids, target, true);
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  return { open, error, clearError: () => setError("") };
}
