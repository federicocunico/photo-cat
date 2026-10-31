import { useTranslation } from "react-i18next";
import type { ModuleId } from "../store/layout";

export function Placeholder({ module }: { module: ModuleId }) {
  const { t } = useTranslation();
  return (
    <div style={{ flex: 1, padding: "24px 32px" }}>
      <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t(`modules.${module}`)}</h1>
      <p className="muted">{t("shell.comingSoon")}</p>
    </div>
  );
}
