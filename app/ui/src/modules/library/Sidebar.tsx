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
import { Disclosure, Menu } from "../../components/ui";
import { RenamePlaceDialog } from "./RenamePlaceDialog";
import { SourceMenu } from "./SourceMenu";
import { updateFolder } from "../import/sourceActions";
import { FacetList } from "./FacetList";
import { DateFacet } from "./DateFacet";
import { catalogLabel } from "../catalogs/systemCatalogs";
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
      className={mono ? "sidebar-row mono" : "sidebar-row"}
      style={indent ? { paddingLeft: 6 + indent } : undefined}
    >
      <span className="sidebar-row-label">
        {icon}
        {label}
      </span>
      <span className="sidebar-row-count">{fmtInt(count)}</span>
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
                <div className="sidebar-tree-row">
                  <span className="sidebar-indent" style={{ width: 8 + depth * 12 }} />
                  {f.hasChildren ? (
                    <BusyButton
                      type="button"
                      className="btn btn-ghost sidebar-toggle"
                      aria-expanded={expanded}
                      aria-label={t(expanded ? "folders.collapse" : "folders.expand", {
                        name: f.name,
                      })}
                      onClick={() => setOpen(toggle(open, f.path))}
                    >
                      {expanded ? <Icon.ChevronDown size={14} /> : <Icon.ChevronRight size={14} />}
                    </BusyButton>
                  ) : (
                    <span className="sidebar-indent-spacer" />
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
                  <Menu
                    align="end"
                    trigger={
                      <button
                        type="button"
                        className="btn btn-ghost icon-btn sidebar-menu"
                        aria-label={t("folders.menu", { name: f.name })}
                      >
                        <Icon.More size={14} aria-hidden="true" />
                      </button>
                    }
                    items={[
                      {
                        label: t("sources.updateFolderOf", { name: f.name }),
                        icon: <Icon.Refresh size={14} />,
                        onSelect: () => updateFolder(source, f.path),
                      },
                    ]}
                  />
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

/**
 * Nome leggibile di una cartella importata: l'ultima parte del percorso, oppure le
 * ultime due se un'altra cartella ha lo stesso nome. Il percorso completo va nel
 * suggerimento.
 */
function folderName(path: string, all: string[]): string {
  const parts = (p: string) => p.split(/[\\/]/).filter(Boolean);
  const last = (p: string) => parts(p).slice(-1)[0] ?? p;
  const name = last(path);
  const clash = all.some((other) => other !== path && last(other) === name);
  return clash
    ? parts(path)
        .slice(-2)
        .join(path.includes("\\") ? "\\" : "/")
    : name;
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
        <div className="sidebar-section-body" style={{ gap }}>
          {children}
        </div>
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
  // Ricerca in tutti i filtri: ogni gruppo mostra solo i valori che corrispondono.
  const [facetQuery, setFacetQuery] = useState("");
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
    <div className="sidebar">
      <SidebarSection id="library.sources" title={t("library.sources")}>
        <QueryView query={sources} enabled={isTauri()} isEmpty={() => false}>
          {(list = []) => (
            <>
              {list.map((summary) => {
                const { source, items } = summary;
                const label = folderName(
                  source.path,
                  list.map((s) => s.source.path),
                );
                const expanded = openSources.includes(source.id);
                return (
                  <div key={source.id}>
                    <div className="sidebar-tree-row">
                      <button
                        type="button"
                        className="btn btn-ghost sidebar-toggle"
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
                <label className="sidebar-check">
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
                  label={catalogLabel(catalog, t)}
                  icon={
                    catalog.kind === "smart" ? (
                      <Icon.Smart size={12} className="sidebar-smart" />
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
        <input
          className="field"
          type="search"
          aria-label={t("library.facets.searchAll")}
          placeholder={t("library.facets.searchAll")}
          value={facetQuery}
          onChange={(e) => setFacetQuery(e.target.value)}
        />
        <label className="sidebar-check">
          <input
            type="checkbox"
            checked={filter.similarOnly ?? false}
            onChange={(e) => setFilter({ similarOnly: e.target.checked })}
          />
          {t("similar.only")}
        </label>
        <QueryView query={facets} enabled={isTauri()} isEmpty={() => false}>
          {(data) =>
            data && (
              <>
                {data.labels.map(([grp, list]) => (
                  <FacetList
                    query={facetQuery}
                    key={grp}
                    facetKey={{ kind: "label", group: grp }}
                    list={list}
                  />
                ))}
                {data.places.total > 0 && (
                  <>
                    <FacetList
                      query={facetQuery}
                      facetKey={{ kind: "place" }}
                      list={data.places}
                      action={(f) => (
                        <button
                          type="button"
                          className="btn btn-ghost facet-icon-btn"
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
                  <FacetList query={facetQuery} facetKey={{ kind: "format" }} list={data.formats} />
                )}
                {data.tags.total > 0 && (
                  <FacetList query={facetQuery} facetKey={{ kind: "tag" }} list={data.tags} />
                )}
                {data.people.total > 0 && (
                  <div className="sidebar-people">
                    <div className="facet-title">{t("library.facets.people")}</div>
                    {data.people.values.slice(0, 8).map((v) => {
                      const id = personIdByName.get(v.value);
                      const active = (filter.personIds ?? []).includes(id ?? -1);
                      return (
                        <div key={v.value} className="sidebar-people-row">
                          <button
                            type="button"
                            className="btn btn-ghost sidebar-people-btn"
                            data-active={active || undefined}
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
                              className="btn btn-ghost facet-icon-btn"
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
