import { useTranslation } from "react-i18next";
import { MODULES } from "../../modules/registry";
import { useLayout } from "../../store/layout";

export function ModuleNav() {
  const { t } = useTranslation();
  const module = useLayout((s) => s.module);
  const setModule = useLayout((s) => s.setModule);
  return (
    <nav
      aria-label={t("modules.navLabel")}
      style={{
        flex: "0 0 200px",
        display: "flex",
        flexDirection: "column",
        gap: 2,
        padding: "12px 8px",
        background: "var(--pc-panel)",
        borderRight: "1px solid var(--pc-border)",
        overflowY: "auto",
      }}
    >
      {MODULES.map(({ id, icon: I, bottom }) => {
        const active = id === module;
        return (
          <button
            key={id}
            type="button"
            aria-current={active ? "page" : undefined}
            onClick={() => setModule(id)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 10,
              padding: "0 12px",
              minHeight: 40,
              borderRadius: 6,
              border: 0,
              textAlign: "left",
              cursor: "pointer",
              background: active ? "var(--pc-raised)" : "transparent",
              color: active ? "var(--pc-text)" : "var(--pc-text-soft)",
              fontWeight: active ? 600 : 400,
              marginTop: bottom ? "auto" : undefined,
            }}
          >
            <I style={{ color: active ? "var(--pc-accent-text)" : "currentColor" }} />
            {t(`modules.${id}`)}
          </button>
        );
      })}
    </nav>
  );
}
