import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { useLibrary } from "../../store/library";

export function CatalogHeader() {
  const { t } = useTranslation();
  const { filter, setFilter, setSelection, setSort } = useLibrary();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const id = filter.excludedFrom ?? filter.catalogId;
  const summary = catalogs.data?.find((c) => c.catalog.id === id);
  if (!summary) return null;
  const excluded = filter.excludedFrom != null;
  const navigate = () => {
    setSelection([], null);
    setFilter({
      catalogId: excluded ? id : null,
      excludedFrom: excluded ? null : id,
      matchingOnly: true,
    });
    setSort(excluded ? "captured_desc" : "excluded_at");
  };
  return (
    <QueryView query={catalogs} enabled={isTauri()} isEmpty={() => false}>
      {() => (
        <div style={{ padding: "8px 16px", borderBottom: "1px solid var(--pc-border)" }}>
          {excluded
            ? t("exclusions.banner", { name: summary.catalog.name })
            : `${summary.catalog.name} · ${summary.items}`}{" "}
          <BusyButton className="btn" onClick={navigate}>
            {excluded ? t("exclusions.back") : t("exclusions.view", { count: summary.excluded })}
          </BusyButton>
          {excluded && summary.catalog.kind === "smart" && (
            <label style={{ marginLeft: 12 }}>
              <input
                type="checkbox"
                checked={filter.matchingOnly === false}
                onChange={(e) => {
                  setSelection([], null);
                  setFilter({ matchingOnly: !e.target.checked });
                }}
              />{" "}
              {t("exclusions.all")}
            </label>
          )}
        </div>
      )}
    </QueryView>
  );
}
