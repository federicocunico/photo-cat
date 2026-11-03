import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useQuery } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import type React from "react";
import { Icon } from "../../components/icons";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type FacetValue, type Source } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { latestOnly, retryUnlessSuperseded } from "../../lib/latest";
import { useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { Disclosure } from "../../components/ui";
import { RenamePlaceDialog } from "./RenamePlaceDialog";
import { SourceMenu } from "./SourceMenu";
import { updateFolder } from "../import/sourceActions";
import { FacetList } from "./FacetList";
import { DateFacet } from "./DateFacet";
import { OpenInPeople } from "../people/OpenInPeople";

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
    <BusyButton
      type="button"
      aria-pressed={active}
      title={title ?? label}
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
    </BusyButton>
  );
}

const toggle = (list: string[], v: string) =>
  list.includes(v) ? list.filter((x) => x !== v) : [...list, v];

/** Sottocartelle di una sorgente (o cartella), caricate un livello alla volta. */
function FolderTree({
  source,
  parent,
  depth,
}: {
  source: Source;
  parent: string | null;
  depth: number;
}) {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const recursive = useLibrary((s) => s.folderRecursive);
  const [open, setOpen] = useState<string[]>([]);
  const folders = useQuery({
    queryKey: ["folders", source.id, parent],
    queryFn: () => commands.listFolders(source.id, parent),
    enabled: isTauri(),
  });
  return (
    <QueryView query={folders} enabled={isTauri()} isEmpty={() => false}>
      {() => (
        <>
          {(folders.data ?? []).map((f) => {
            const expanded = open.includes(f.path);
            return (
              <div key={f.path}>
                <div style={{ display: "flex", alignItems: "center" }}>
                  <span style={{ width: 8 + depth * 12, flexShrink: 0 }} />
                  {f.hasChildren ? (
                    <BusyButton
                      type="button"
                      className="btn btn-ghost"
                      style={{ padding: "0 4px", fontSize: 11, width: 20 }}
                      aria-expanded={expanded}
                      aria-label={t(expanded ? "folders.collapse" : "folders.expand", {
                        name: f.name,
                      })}
                      onClick={() => setOpen(toggle(open, f.path))}
                    >
                      {expanded ? <Icon.ChevronDown size={14} /> : <Icon.ChevronRight size={14} />}
                    </BusyButton>
                  ) : (
                    <span style={{ width: 20, flexShrink: 0 }} />
                  )}
                  <Row
                    mono
                    title={f.path}
                    label={f.name}
                    count={f.items}
                    active={filter.folder?.path === f.path}
                    onClick={() =>
                      setFilter({
                        folder: filter.folder?.path === f.path ? null : { path: f.path, recursive },
                      })
                    }
                  />
                  <BusyButton
                    type="button"
                    className="btn btn-ghost"
                    style={{ padding: "0 4px", fontSize: 12 }}
                    title={t("sources.updateFolder")}
                    aria-label={t("sources.updateFolderOf", { name: f.name })}
                    onClick={async () => await updateFolder(source, f.path)}
                  >
                    <Icon.Refresh size={14} />
                  </BusyButton>
                </div>
                {expanded && <FolderTree source={source} parent={f.path} depth={depth + 1} />}
              </div>
            );
          })}
        </>
      )}
    </QueryView>
  );
}

/** Sezione del pannello sinistro richiudibile; lo stato è ricordato tra i riavvii. */
function SidebarSection({
  id,
  title,
  gap = 4,
  children,
}: {
  id: string;
  title: string;
  gap?: number;
  children: ReactNode;
}) {
  const collapsed = useLayout((s) => s.collapsedSections.includes(id));
  const toggle = useLayout((s) => s.toggleSection);
  return (
    <section aria-label={title}>
      <Disclosure title={title} level={2} open={!collapsed} onOpenChange={() => toggle(id)}>
        <div style={{ display: "flex", flexDirection: "column", gap }}>{children}</div>
      </Disclosure>
    </section>
  );
}

export function LibrarySidebar() {
  const { t } = useTranslation();
  const filter = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const folderRecursive = useLibrary((s) => s.folderRecursive);
  const setFolderRecursive = useLibrary((s) => s.setFolderRecursive);
  const [openSources, setOpenSources] = useState<number[]>([]);
  const [renaming, setRenaming] = useState<FacetValue | null>(null);
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
    queryFn: () => latestOnly("facets", (tag) => commands.libraryFacets(filter, tag)),
    enabled: isTauri(),
    retry: retryUnlessSuperseded,
    placeholderData: (prev) => prev,
  });
  const people = useQuery({
    queryKey: ["people"],
    queryFn: commands.listPeople,
    enabled: isTauri() && (facets.data?.people.total ?? 0) > 0,
  });
  const personIdByName = new Map(
    (people.data ?? []).filter((p) => p.name).map((p) => [p.name as string, p.id] as const),
  );

  // Ogni sezione ha il proprio stato di caricamento: una query lenta dei filtri non
  // nasconde cartelle e cataloghi.
  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 20 }}>
      <SidebarSection id="library.sources" title={t("library.sources")}>
        <QueryView query={sources} enabled={isTauri()} isEmpty={() => false}>
          {(list = []) => (
            <>
              {list.map((summary) => {
                const { source, items } = summary;
                const label = source.path
                  .split(/[\\/]/)
                  .filter(Boolean)
                  .slice(-2)
                  .join(source.path.includes("\\") ? "\\" : "/");
                const expanded = openSources.includes(source.id);
                return (
                  <div key={source.id}>
                    <div style={{ display: "flex", alignItems: "center" }}>
                      <button
                        type="button"
                        className="btn btn-ghost"
                        style={{ padding: "0 4px", fontSize: 11, width: 20 }}
                        aria-expanded={expanded}
                        aria-label={t(expanded ? "folders.collapse" : "folders.expand", {
                          name: label,
                        })}
                        onClick={() =>
                          setOpenSources(
                            expanded
                              ? openSources.filter((x) => x !== source.id)
                              : [...openSources, source.id],
                          )
                        }
                      >
                        {expanded ? (
                          <Icon.ChevronDown size={14} />
                        ) : (
                          <Icon.ChevronRight size={14} />
                        )}
                      </button>
                      <Row
                        mono
                        title={source.path}
                        label={label}
                        count={items}
                        active={(filter.sourceIds ?? []).includes(source.id)}
                        onClick={() =>
                          setFilter({
                            sourceIds: (filter.sourceIds ?? []).includes(source.id)
                              ? []
                              : [source.id],
                          })
                        }
                      />
                      <SourceMenu summary={summary} label={label} />
                    </div>
                    {expanded && <FolderTree source={source} parent={null} depth={0} />}
                  </div>
                );
              })}
              {openSources.length > 0 && (
                <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={folderRecursive}
                    onChange={(e) => setFolderRecursive(e.target.checked)}
                  />
                  {t("folders.includeSub")}
                </label>
              )}
            </>
          )}
        </QueryView>
      </SidebarSection>
      <SidebarSection id="library.catalogs" title={t("library.catalogs")}>
        <QueryView query={catalogs} enabled={isTauri()} isEmpty={() => false}>
          {(list = []) => (
            <>
              {list.map(({ catalog, items }) => (
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
            </>
          )}
        </QueryView>
      </SidebarSection>
      <SidebarSection id="library.filters" title={t("library.filters")} gap={12}>
        <QueryView query={facets} enabled={isTauri()} isEmpty={() => false}>
          {(data) =>
            data && (
              <>
                {data.labels.map(([grp, list]) => (
                  <FacetList key={grp} facetKey={{ kind: "label", group: grp }} list={list} />
                ))}
                {data.places.total > 0 && (
                  <>
                    <FacetList
                      facetKey={{ kind: "place" }}
                      list={data.places}
                      action={(f) => (
                        <button
                          type="button"
                          className="btn btn-ghost"
                          style={{ padding: "0 4px", fontSize: 12 }}
                          title={t("place.rename")}
                          aria-label={t("place.renameOf", { name: f.value })}
                          onClick={() => setRenaming(f)}
                        >
                          <Icon.Edit size={14} />
                        </button>
                      )}
                    />
                    {renaming && (
                      <RenamePlaceDialog
                        key={renaming.value}
                        from={renaming.value}
                        count={renaming.count}
                        places={data.places.values}
                        onClose={() => setRenaming(null)}
                      />
                    )}
                  </>
                )}
                {data.years.total > 0 && (
                  <DateFacet
                    years={data.years}
                    months={data.months ?? null}
                    days={data.days ?? null}
                  />
                )}
                {data.formats.total > 0 && (
                  <FacetList facetKey={{ kind: "format" }} list={data.formats} />
                )}
                {data.tags.total > 0 && <FacetList facetKey={{ kind: "tag" }} list={data.tags} />}
                {data.people.total > 0 && (
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    <div style={{ fontWeight: 600 }}>{t("library.facets.people")}</div>
                    {data.people.values.slice(0, 8).map((v) => {
                      const id = personIdByName.get(v.value);
                      return (
                        <div
                          key={v.value}
                          style={{ display: "flex", alignItems: "center", gap: 4, minHeight: 24 }}
                        >
                          <button
                            type="button"
                            className="btn btn-ghost"
                            style={{
                              flex: 1,
                              justifyContent: "space-between",
                              padding: "0 4px",
                              fontWeight: (filter.personIds ?? []).includes(id ?? -1)
                                ? 600
                                : undefined,
                            }}
                            onClick={() => {
                              if (id == null) return;
                              const cur = filter.personIds ?? [];
                              setFilter({
                                personIds: cur.includes(id)
                                  ? cur.filter((x) => x !== id)
                                  : [...cur, id],
                              });
                            }}
                          >
                            <span>{v.value}</span>
                            <span className="muted">{fmtInt(v.count)}</span>
                          </button>
                          {id != null && (
                            <OpenInPeople
                              target={{ person: id }}
                              style={{ padding: "0 4px", fontSize: 12 }}
                            />
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </>
            )
          }
        </QueryView>
      </SidebarSection>
    </div>
  );
}
