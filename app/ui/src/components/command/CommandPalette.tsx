// Palette dei comandi (command-palette): un solo elenco raggruppato per tipo con
// comandi, persone, tag, cataloghi, luoghi e nomi di file. Ctrl+K o il campo della barra
// superiore la aprono; si usa solo da tastiera.
import { Command, useCommandState } from "cmdk";
import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  availableCommands,
  shortcutText,
  useCommandStore,
  type Command as AppCommand,
} from "../../commands/registry";
import { commands, isTauri, type LibraryFilter } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { usePalette } from "../../store/palette";
import { Icon } from "../icons";
import { catalogLabel } from "../../modules/catalogs/systemCatalogs";
import "./palette.css";

/** Risultati per gruppo di entità. */
const ENTITY_LIMIT = 8;
/** Nomi di file: i primi risultati del filtro di testo (design D7). */
const FILE_LIMIT = 20;

function useDebounced(value: string, ms = 200) {
  const [v, setV] = useState(value);
  useEffect(() => {
    const h = setTimeout(() => setV(value), ms);
    return () => clearTimeout(h);
  }, [value, ms]);
  return v;
}

/**
 * Corrispondenza per parti di testo, senza maiuscole: prevedibile ("pers" → Persone) e
 * senza le corrispondenze sparse della ricerca approssimata.
 */
function contains(value: string, search: string, keywords?: string[]) {
  const text = [value, ...(keywords ?? [])].join(" ").toLocaleLowerCase();
  return text.includes(search.trim().toLocaleLowerCase()) ? 1 : 0;
}

/** Apre la Libreria con il filtro dato (sostituisce quello attuale). */
function openLibrary(patch: Partial<LibraryFilter>, select?: number) {
  const lib = useLibrary.getState();
  lib.setFilter({ ...emptyFilter(), ...patch });
  if (select != null) lib.setSelection([select], select);
  useLayout.getState().openModule({ module: "library" });
}

/** Solo la ricerca nei nomi dei file corrisponde: lo si dice esplicitamente. */
function NoResults() {
  const { t } = useTranslation();
  const count = useCommandState((s) => s.filtered.count);
  return count <= 1 ? <div className="palette-empty">{t("palette.empty")}</div> : null;
}

export function CommandPalette() {
  const { t } = useTranslation();
  const open = usePalette((s) => s.open);
  const initial = usePalette((s) => s.initial);
  const recent = usePalette((s) => s.recent);
  const close = usePalette((s) => s.close);
  const remember = usePalette((s) => s.remember);
  // Si rilegge il registro all'apertura: i comandi disponibili dipendono dallo stato.
  useCommandStore((s) => s.commands);
  const [search, setSearch] = useState("");
  useEffect(() => {
    if (open) setSearch(initial);
  }, [open, initial]);
  const text = search.trim();
  const q = useDebounced(text);

  const people = useQuery({
    queryKey: ["people"],
    queryFn: () => commands.listPeople(),
    enabled: isTauri() && open,
  });
  const tags = useQuery({
    queryKey: ["tags"],
    queryFn: () => commands.tagOverview(),
    enabled: isTauri() && open,
  });
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri() && open,
  });
  const places = useQuery({
    queryKey: ["place-suggest", q],
    queryFn: () => commands.suggestPlaces(q),
    enabled: isTauri() && open && q.length > 0,
  });
  const files = useQuery({
    queryKey: ["palette-files", q],
    queryFn: () =>
      commands.libraryPage({ ...emptyFilter(), text: q }, "name_asc", null, FILE_LIMIT),
    enabled: isTauri() && open && q.length > 1,
  });

  const run = (fn: () => unknown, id?: string) => {
    if (id) remember(id);
    close();
    void fn();
  };
  const list = availableCommands();
  const byGroup = new Map<string, AppCommand[]>();
  for (const c of list) byGroup.set(c.group, [...(byGroup.get(c.group) ?? []), c]);
  const recentCommands = recent
    .map((id) => list.find((c) => c.id === id))
    .filter((c): c is AppCommand => c != null);
  const match = (text: string) => text.toLocaleLowerCase().includes(q.toLocaleLowerCase());

  const commandItem = (c: AppCommand, prefix = "") => (
    <Command.Item
      key={`${prefix}${c.id}`}
      value={`${prefix}${c.id} ${c.label}`}
      keywords={[c.label, c.keywords ?? ""]}
      onSelect={() => run(() => c.run?.(), c.id)}
    >
      <span className="palette-label">{c.label}</span>
      {(c.shortcut || c.keys) && (
        <kbd className="palette-keys">{c.keys ?? shortcutText(c.shortcut!)}</kbd>
      )}
    </Command.Item>
  );

  return (
    <Command.Dialog
      open={open}
      onOpenChange={(o) => !o && close()}
      label={t("palette.label")}
      overlayClassName="palette-overlay"
      filter={contains}
      contentClassName="palette"
      loop
    >
      <div className="palette-input">
        <Icon.Search size={16} aria-hidden="true" />
        <Command.Input
          value={search}
          onValueChange={setSearch}
          placeholder={t("palette.placeholder")}
          aria-label={t("palette.placeholder")}
        />
      </div>
      <Command.List className="palette-list">
        <Command.Empty className="palette-empty">{t("palette.empty")}</Command.Empty>
        {text && <NoResults />}
        {!q && recentCommands.length > 0 && (
          <Command.Group heading={t("palette.recent")}>
            {recentCommands.map((c) => commandItem(c, "recent:"))}
          </Command.Group>
        )}
        {[...byGroup.entries()].map(([group, cmds]) => (
          <Command.Group key={group} heading={t(`commands.groups.${group}`)}>
            {cmds.map((c) => commandItem(c))}
          </Command.Group>
        ))}
        {q && (
          <>
            <Command.Group heading={t("palette.people")}>
              {(people.data ?? [])
                .filter((p) => p.name && match(p.name))
                .slice(0, ENTITY_LIMIT)
                .map((p) => (
                  <Command.Item
                    key={`p${p.id}`}
                    value={`person:${p.id} ${p.name}`}
                    onSelect={() => run(() => openLibrary({ personIds: [p.id] }))}
                  >
                    <Icon.People size={14} aria-hidden="true" />
                    <span className="palette-label">{p.name}</span>
                  </Command.Item>
                ))}
            </Command.Group>
            <Command.Group heading={t("palette.tags")}>
              {(tags.data?.tags ?? [])
                .filter((x) => match(x.name))
                .slice(0, ENTITY_LIMIT)
                .map((x) => (
                  <Command.Item
                    key={`t${x.name}`}
                    value={`tag:${x.name}`}
                    onSelect={() => run(() => openLibrary({ tags: [x.name] }))}
                  >
                    <Icon.Tags size={14} aria-hidden="true" />
                    <span className="palette-label">{x.name}</span>
                  </Command.Item>
                ))}
            </Command.Group>
            <Command.Group heading={t("palette.catalogs")}>
              {(catalogs.data ?? [])
                .filter((c) => match(catalogLabel(c.catalog, t)))
                .slice(0, ENTITY_LIMIT)
                .map(({ catalog: c }) => (
                  <Command.Item
                    key={`c${c.id}`}
                    value={`catalog:${c.id} ${catalogLabel(c, t)}`}
                    onSelect={() => run(() => openLibrary({ catalogId: c.id }))}
                  >
                    <Icon.Catalogs size={14} aria-hidden="true" />
                    <span className="palette-label">{catalogLabel(c, t)}</span>
                  </Command.Item>
                ))}
            </Command.Group>
            <Command.Group heading={t("palette.places")}>
              {(places.data ?? [])
                .filter((p) => p.inLibrary)
                .slice(0, ENTITY_LIMIT)
                .map((p) => (
                  <Command.Item
                    key={`l${p.name}`}
                    value={`place:${p.name}`}
                    keywords={[q]}
                    onSelect={() => run(() => openLibrary({ places: [p.name] }))}
                  >
                    <span className="palette-label">{p.name}</span>
                  </Command.Item>
                ))}
            </Command.Group>
            <Command.Group heading={t("palette.files")}>
              {(files.data?.items ?? []).map((f) => (
                <Command.Item
                  key={`f${f.id}`}
                  value={`file:${f.id} ${f.name}`}
                  keywords={[q]}
                  onSelect={() => run(() => openLibrary({ text: q }, f.id))}
                >
                  <span className="palette-label mono">{f.name}</span>
                </Command.Item>
              ))}
            </Command.Group>
          </>
        )}
        {text && (
          // Invio sul testo libero: lo cerca come testo in Libreria.
          <Command.Item
            value={`search:${text}`}
            keywords={[text]}
            onSelect={() => run(() => openLibrary({ text }))}
          >
            <Icon.Search size={14} aria-hidden="true" />
            <span className="palette-label">{t("palette.searchText", { text })}</span>
          </Command.Item>
        )}
      </Command.List>
    </Command.Dialog>
  );
}
