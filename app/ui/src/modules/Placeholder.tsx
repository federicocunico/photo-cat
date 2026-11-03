import { useTranslation } from "react-i18next";
import type { ModuleId } from "../store/layout";
import "../components/shell/shell.css";

export function Placeholder({ module }: { module: ModuleId }) {
  const { t } = useTranslation();
  return (
    <div className="module-placeholder">
      <h1>{t(`modules.${module}`)}</h1>
      <p className="muted">{t("shell.comingSoon")}</p>
    </div>
  );
}
