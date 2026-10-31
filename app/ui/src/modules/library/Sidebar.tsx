import { useQuery } from "@tanstack/react-query";
import type React from "react";
import { Icon } from "../../components/icons";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type FacetValue, type LibraryFilter } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLibrary } from "../../store/library";

function Row({
  label,
  count,
  active,
  onClick,
  mono,
  indent = 0,
  icon,
  title,
}: {
  icon?: React.ReactNode;
  title?: string;
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
  mono?: boolean;
  indent?: number;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      title={title}
      onClick={onClick}
      className={mono ? "mono" : undefined}
      style={{
        display: "flex",
        justifyContent: "space-between",
        gap: 8,
        width: "100%",
        border: 0,
        cursor: "pointer",
        textAlign: "left",
        padding: `3px 6px 3px ${6 + indent}px`,
        borderRadius: 4,
        fontSize: mono ? 12 : 14,
        background: active ? "var(--pc-tree-selected)" : "transparent",
        color: "var(--pc-text)",
      }}
    >
      <span
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {icon}
        {label}
      </span>
      <span style={{ color: active ? "var(--pc-accent-soft-text)" : "var(--pc-text-secondary)" }}>
        {fmtInt(count)}
      </span>
    </button>
  );
}

function CheckList({
  values,
  selected,
  onToggle,
  render = (v) => v,
}: {
  values: FacetValue[];
  selected: string[];
  onToggle: (v: string) => void;
  render?: (v: string) => string;
}) {
  return (
    <>
      {values.slice(0, 12).map((f) => (
        <label key={f.value} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <input
            type="checkbox"
            checked={selected.includes(f.value)}
            onChange={() => onToggle(f.value)}
          />
          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {render(f.value)}
          </span>
          <span className="muted" style={{ marginLeft: "auto" }}>
            {fmtInt(f.count)}
          </span>
        </label>
      ))}
    </>
  );
}

const toggle = (list: string[], v: string) =>
  list.includes(v) ? list.filter((x) => x !== v) : [...list, v];

export function LibrarySidebar() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: commands.listSources,
    enabled: isTauri(),
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: commands.listCatalogs,
    enabled: isTauri(),
  });
  const facets = useQuery({
    queryKey: ["facets", filter],
    queryFn: () => commands.libraryFacets(filter),
    enabled: isTauri(),
    placeholderData: (prev) => prev,
  });
  const labelSel = (grp: string) => filter.labels?.find((l) => l.group === grp)?.values ?? [];
  const setLabel = (grp: string, values: string[]) => {
    const others = (filter.labels ?? []).filter((l) => l.group !== grp);
    setFilter({ labels: values.length ? [...others, { group: grp, values }] : others });
  };
  const years = (filter.dateFrom ?? "").slice(0, 4);
  const setYear = (y: string) =>
    setFilter(
      years === y
        ? { dateFrom: null, dateTo: null }
        : ({ dateFrom: `${y}-01-01`, dateTo: `${y}-12-31` } as Partial<LibraryFilter>),
    );

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 20 }}>
      <section style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <h2 className="section-title">{t("library.sources")}</h2>
        {(sources.data ?? []).map(({ source, items }) => (
          <Row
            key={source.id}
            mono
            title={source.path}
            label={source.path.split(/[\\/]/).filter(Boolean).slice(-2).join("\\")}
            count={items}
            active={(filter.sourceIds ?? []).includes(source.id)}
            onClick={() =>
              setFilter({
                sourceIds: (filter.sourceIds ?? []).includes(source.id) ? [] : [source.id],
              })
            }
          />
        ))}
      </section>
      <section style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <h2 className="section-title">{t("library.catalogs")}</h2>
        {(catalogs.data ?? []).map(({ catalog, items }) => (
          <Row
            key={catalog.id}
            label={catalog.name}
            icon={
              catalog.kind === "smart" ? (
                <Icon.Smart size={12} style={{ color: "var(--pc-warn)" }} />
              ) : undefined
            }
            count={items}
            active={filter.catalogId === catalog.id}
            onClick={() =>
              setFilter({ catalogId: filter.catalogId === catalog.id ? null : catalog.id })
            }
          />
        ))}
      </section>
      <section
        style={{ display: "flex", flexDirection: "column", gap: 8 }}
        aria-label={t("library.filters")}
      >
        <h2 className="section-title">{t("library.filters")}</h2>
        {(facets.data?.labels ?? []).map(([grp, values]) => (
          <div key={grp} style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ fontWeight: 600 }}>
              {t(`library.facets.groups.${grp}`, { defaultValue: grp })}
            </div>
            <CheckList
              values={values}
              selected={labelSel(grp)}
              onToggle={(v) => setLabel(grp, toggle(labelSel(grp), v))}
              render={(v) => t(`labels.${v}`, { defaultValue: v.replace(/_/g, " ") })}
            />
          </div>
        ))}
        {(facets.data?.places.length ?? 0) > 0 && (
          <>
            <div style={{ fontWeight: 600 }}>{t("library.facets.places")}</div>
            <CheckList
              values={facets.data!.places}
              selected={filter.places ?? []}
              onToggle={(v) => setFilter({ places: toggle(filter.places ?? [], v) })}
            />
          </>
        )}
        {(facets.data?.years.length ?? 0) > 0 && (
          <>
            <div style={{ fontWeight: 600 }}>{t("library.facets.years")}</div>
            <CheckList
              values={facets.data!.years}
              selected={years ? [years] : []}
              onToggle={setYear}
            />
          </>
        )}
        {(facets.data?.formats.length ?? 0) > 0 && (
          <>
            <div style={{ fontWeight: 600 }}>{t("library.facets.formats")}</div>
            <CheckList
              values={facets.data!.formats}
              selected={filter.formats ?? []}
              onToggle={(v) => setFilter({ formats: toggle(filter.formats ?? [], v) })}
              render={(v) => v.toUpperCase()}
            />
          </>
        )}
        {(facets.data?.tags.length ?? 0) > 0 && (
          <>
            <div style={{ fontWeight: 600 }}>{t("library.facets.tags")}</div>
            <CheckList
              values={facets.data!.tags}
              selected={filter.tags ?? []}
              onToggle={(v) => setFilter({ tags: toggle(filter.tags ?? [], v) })}
            />
          </>
        )}
      </section>
    </div>
  );
}
