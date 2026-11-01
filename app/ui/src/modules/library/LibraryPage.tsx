import { useTranslation } from "react-i18next";
import { Panels } from "../../components/shell/Panels";
import type { SortKey } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { activeChips } from "./chips";
import { LibraryGrid } from "./Grid";
import { Inspector } from "./Inspector";
import { LibrarySidebar } from "./Sidebar";
import { BulkBar } from "./BulkBar";
import { useTagHotkeys } from "../tags/useTagHotkeys";

const SORTS: SortKey[] = ["captured_desc", "captured_asc", "name_asc", "size_desc", "added_desc"];

function Toolbar() {
  const { t } = useTranslation();
  const { filter, sort, setSort, thumbSize, setThumbSize, setFilter, resetFilter } = useLibrary();
  const chips = activeChips(filter, t as never);
  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: 10,
        padding: "10px 16px",
        borderBottom: "1px solid var(--pc-border)",
        background: "var(--pc-toolbar)",
      }}
    >
      <label className="muted" style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {t("library.sort")}
        <select className="field" value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
          {SORTS.map((s) => (
            <option key={s} value={s}>
              {t(`library.sortKeys.${s}`)}
            </option>
          ))}
        </select>
      </label>
      <label className="muted" style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {t("library.thumbSize")}
        <input
          type="range"
          min={1}
          max={5}
          value={thumbSize}
          aria-label={t("library.thumbSize")}
          onChange={(e) => setThumbSize(Number(e.target.value))}
        />
      </label>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginLeft: "auto" }}>
        {chips.map((c) => (
          <button
            key={c.name}
            type="button"
            className="chip"
            style={{ border: 0, cursor: "pointer" }}
            aria-label={t("library.removeFilter", { name: c.name })}
            onClick={() => setFilter(c.clear)}
          >
            {c.name} <span aria-hidden="true">×</span>
          </button>
        ))}
        {chips.length > 1 && (
          <button type="button" className="btn btn-ghost" onClick={resetFilter}>
            {t("library.clearFilters")}
          </button>
        )}
      </div>
    </div>
  );
}

export function LibraryPage() {
  useTagHotkeys(() => useLibrary.getState().selected);
  return (
    <Panels
      left={<LibrarySidebar />}
      center={
        <>
          <Toolbar />
          <BulkBar />
          <LibraryGrid />
        </>
      }
      right={<Inspector />}
    />
  );
}
