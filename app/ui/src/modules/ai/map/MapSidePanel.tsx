// Pannello laterale della mappa (photo-map D5): ambito, cataloghi nell'area, luoghi senza coordinate.
import { useTranslation } from "react-i18next";
import type { Catalog, CatalogCount, Cell, PlaceCount, PlacePosition } from "../../../bindings";
import { Button, InlineAlert } from "../../../components/ui";
import { useLayout } from "../../../store/layout";
import { cellLabel, isApproxCluster } from "./clusters";

const ALL = "all";
const MAX_GROUPS = 30;

export interface MapSidePanelProps {
  scope: number | null;
  onScope: (id: number | null) => void;
  catalogs: Catalog[];
  inArea: CatalogCount[];
  unplaced: PlaceCount[];
  manual: PlacePosition[];
  onClear: (name: string) => void;
  placing: string | null;
  onPlace: (name: string | null) => void;
  gazetteerMissing: boolean;
  /** Gruppi (celle) dell'area visibile. */
  cells: Cell[];
  onOpenCell: (cell: Cell) => void;
}

export function MapSidePanel(p: MapSidePanelProps) {
  const { t } = useTranslation();
  const scopes = p.catalogs.filter((c) => !c.system);
  // Alternativa da tastiera ai cerchi della mappa: i gruppi dell'area, dal più grande.
  const groups = [...p.cells].sort((a, b) => b.count - a.count).slice(0, MAX_GROUPS);
  return (
    <aside className="ai-map-panel" aria-label={t("aiMap.panel")}>
      <label className="ai-results-field">
        <span>{t("aiMap.scope")}</span>
        <select
          value={p.scope === null ? ALL : String(p.scope)}
          onChange={(e) => p.onScope(e.target.value === ALL ? null : Number(e.target.value))}
        >
          <option value={ALL}>{t("aiMap.scopeAll")}</option>
          {scopes.map((c) => (
            <option key={c.id} value={String(c.id)}>
              {c.name}
            </option>
          ))}
        </select>
      </label>

      <section>
        <h3>{t("aiMap.groupsInArea")}</h3>
        {groups.length === 0 ? (
          <p className="muted">{t("aiMap.groupsEmpty")}</p>
        ) : (
          <ul className="ai-map-list ai-map-groups">
            {groups.map((c) => (
              <li key={`${c.cx}:${c.cy}`}>
                <button
                  type="button"
                  className="ai-map-row"
                  aria-label={t("aiMap.openGroup", { name: cellLabel(c) || t("aiMap.noPlace"), count: c.count })}
                  onClick={() => p.onOpenCell(c)}
                >
                  <span>
                    {cellLabel(c) || t("aiMap.noPlace")}
                    {isApproxCluster(c) && <span className="muted"> ≈</span>}
                  </span>
                  <span className="muted">{c.count}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h3>{t("aiMap.catalogsInArea")}</h3>
        {p.inArea.length === 0 ? (
          <p className="muted">{t("aiMap.catalogsEmpty")}</p>
        ) : (
          <ul className="ai-map-list">
            {p.inArea.map((c) => (
              <li key={c.catalogId}>
                <button
                  type="button"
                  className="ai-map-row"
                  aria-pressed={p.scope === c.catalogId}
                  onClick={() => p.onScope(c.catalogId)}
                >
                  <span>{c.name}</span>
                  <span className="muted">{c.count}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h3>{t("aiMap.unplaced")}</h3>
        {p.gazetteerMissing && (
          <InlineAlert
            tone="info"
            action={
              <Button onClick={() => useLayout.getState().openModule({ module: "settings", section: "models" })}>
                {t("aiMap.openModels")}
              </Button>
            }
          >
            {t("aiMap.gazetteerHint")}
          </InlineAlert>
        )}
        {p.placing && (
          <InlineAlert tone="info" action={<Button onClick={() => p.onPlace(null)}>{t("aiMap.cancel")}</Button>}>
            {t("aiMap.placingHint", { name: p.placing })}
          </InlineAlert>
        )}
        {p.unplaced.length === 0 ? (
          <p className="muted">{t("aiMap.unplacedEmpty")}</p>
        ) : (
          <ul className="ai-map-list">
            {p.unplaced.map((u) => (
              <li key={u.name} className="ai-map-row">
                <span>
                  {u.name} <span className="muted">{u.count}</span>
                </span>
                <Button
                  aria-label={t("aiMap.placeNamed", { name: u.name })}
                  onClick={() => p.onPlace(p.placing === u.name ? null : u.name)}
                >
                  {t("aiMap.place")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h3>{t("aiMap.manual")}</h3>
        {p.manual.length === 0 ? (
          <p className="muted">{t("aiMap.manualEmpty")}</p>
        ) : (
          <ul className="ai-map-list">
            {p.manual.map((m) => (
              <li key={m.name} className="ai-map-row">
                <span>
                  {m.name} <span className="muted">{m.count}</span>
                </span>
                <span className="ai-map-actions">
                  <Button
                    aria-label={t("aiMap.moveNamed", { name: m.name })}
                    onClick={() => p.onPlace(p.placing === m.name ? null : m.name)}
                  >
                    {t("aiMap.move")}
                  </Button>
                  <Button aria-label={t("aiMap.clearNamed", { name: m.name })} onClick={() => p.onClear(m.name)}>
                    {t("aiMap.clear")}
                  </Button>
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </aside>
  );
}
