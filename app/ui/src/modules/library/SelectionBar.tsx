// Barra delle azioni sulla selezione (desktop-shell, "Barra delle azioni sulla selezione"
// e "Selettori delle azioni sulla selezione"). Sostituisce LibraryHeader quando c'è una
// selezione; le azioni restano quelle di bulk-operations.
import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import {
  Button,
  ConfirmDialog,
  Dialog,
  FormField,
  IconButton,
  Menu,
  Popover,
  SearchPicker,
  type MenuItem,
} from "../../components/ui";
import { commands, isTauri, type BulkAction, type Target } from "../../lib/ipc";
import { useCommands } from "../../commands/registry";
import { selCount, selTarget, useLibrary } from "../../store/library";
import { useLayout } from "../../store/layout";
import { notify, notifyError } from "../../store/toasts";
import { useTagOverview } from "../tags/data";
import { CategoryPicker } from "./CategoryPicker";
import { useLabelName, useTaxonomy } from "./labelName";
import { useCatalogAction } from "./useCatalogAction";
import { useOpenWith } from "./useOpenWith";
import { useSelectItems } from "./useSelectItems";
import { useSelectionPicker } from "../../store/palette";
import { catalogLabel, systemCatalogId } from "../catalogs/systemCatalogs";
import { useBulkJob } from "./bulkJob";
import { useMoveToTrash } from "./useMoveToTrash";

type DoneKind =
  | "trash"
  | "restore_from_system"
  | "add_tag"
  | "remove_tag"
  | "add_catalog"
  | "set_label"
  | "set_place"
  | "reset_place"
  | "assign_person"
  | "remove_library";

/** Esc svuota la selezione, salvo nei campi di testo e con menu o dialoghi aperti. */
function useEscapeToClear(clear: () => void) {
  const { t } = useTranslation();
  useCommands([
    {
      id: "selection.clear",
      label: t("commands.clearSelection"),
      group: "selection",
      shortcut: { key: "Escape" },
      // Un menu o un selettore aperto si chiude prima con il suo Esc.
      when: () => !document.querySelector("[data-radix-popper-content-wrapper]"),
      run: clear,
    },
  ]);
}

export function SelectMenu() {
  const { t } = useTranslation();
  const items = useSelectItems();
  return (
    <Menu
      align="end"
      trigger={
        <button type="button" className="btn btn-ghost btn-sm">
          {t("selection.select")}
          <Icon.ChevronDown size={14} aria-hidden="true" />
        </button>
      }
      items={items}
    />
  );
}

/** Comando della barra che apre un selettore con ricerca accanto a sé. */
function PickerButton({
  label,
  icon,
  children,
  open: controlled,
  onOpenChange,
}: {
  label: string;
  icon: React.ReactNode;
  children: (close: () => void) => React.ReactNode;
  /** Aperto da fuori (es. "Aggiungi tag…" nella palette). */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  const [inner, setInner] = useState(false);
  const open = controlled ?? inner;
  const setOpen = (o: boolean) => (onOpenChange ? onOpenChange(o) : setInner(o));
  return (
    <Popover
      label={label}
      open={open}
      onOpenChange={setOpen}
      trigger={
        <button type="button" className="btn btn-ghost">
          {icon}
          {label}
        </button>
      }
    >
      {children(() => setOpen(false))}
    </Popover>
  );
}

export function SelectionBar() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { selection, filter, setSelection } = useLibrary();
  const count = selCount(selection);
  const target = selTarget(selection);
  const ids = selection.mode === "ids" ? [...selection.ids] : null;
  const bulk = useBulkJob();
  const trash = useMoveToTrash();
  const catalogAction = useCatalogAction();
  const openWith = useOpenWith();
  const selectItems = useSelectItems();
  const tags = useTagOverview();
  const catalogs = useQuery({
    queryKey: ["catalogs"],
    queryFn: () => commands.listCatalogs(),
    enabled: isTauri(),
  });
  const people = useQuery({
    queryKey: ["people"],
    queryFn: () => commands.listPeople(),
    enabled: isTauri(),
  });
  const config = useQuery({
    queryKey: ["config"],
    queryFn: () => commands.getConfig(),
    enabled: isTauri(),
  });
  const [removing, setRemoving] = useState(false);
  const [assigning, setAssigning] = useState<{ id: number; name: string } | null>(null);
  const [categoryOpen, setCategoryOpen] = useState(false);
  const [removeTagOpen, setRemoveTagOpen] = useState(false);
  const clear = () => setSelection([]);
  useEscapeToClear(clear);
  const picker = useSelectionPicker();
  useCommands([
    {
      id: "selection.addTag",
      label: t("commands.addTag"),
      group: "selection",
      when: () => selCount(useLibrary.getState().selection) > 0,
      run: () => picker.set("tag"),
    },
    {
      id: "selection.export",
      label: t("commands.exportSelection"),
      group: "selection",
      when: () => selCount(useLibrary.getState().selection) > 0,
      run: () => useLayout.getState().openModule({ module: "organize" }),
    },
  ]);

  const start = (operation: BulkAction, kind: DoneKind, name = "") =>
    bulk.start(target, operation, t(`selection.done.${kind}`, { name, count }));

  const tagOptions = (tags.data?.tags ?? []).map((x) => ({
    value: x.name,
    label: x.name,
    hint: t("library.count", { n: x.count }),
  }));
  // Catalogo: aggiungere ai manuali, escludere da tutti; nel catalogo aperto il comando diretto.
  const openId = filter.excludedFrom ?? filter.catalogId;
  const openCatalog = catalogs.data?.find((c) => c.catalog.id === openId)?.catalog;
  const trashId = systemCatalogId(catalogs.data, "trash");
  const catalogOptions = (catalogs.data ?? [])
    .filter(({ catalog: c }) => !c.system)
    .flatMap(({ catalog: c }) => [
      ...(c.kind === "manual"
        ? [{ value: `add:${c.id}`, label: t("selection.addTo", { name: c.name }) }]
        : []),
      { value: `exclude:${c.id}`, label: t("selection.excludeFrom", { name: c.name }) },
    ]);
  const pickCatalog = (value: string) => {
    const [op, raw] = value.split(":");
    const id = Number(raw);
    const name = catalogs.data?.find((c) => c.catalog.id === id)?.catalog.name ?? "";
    if (op === "add") return start({ action: "add_catalog", catalog: id }, "add_catalog", name);
    return catalogAction.act({ target, count }, id, name);
  };
  const peopleOptions = (people.data ?? [])
    .filter((p) => p.name)
    .map((p) => ({
      value: String(p.id),
      label: p.name as string,
      hint: t("library.count", { n: p.photos }),
    }));
  const apps = config.data?.external_apps ?? [];
  const openWithItems: MenuItem[] = [
    ...apps.map((a) => ({
      label: a.name,
      onSelect: () => ids && openWith.open(ids, { kind: "app", id: a.id ?? "" }),
    })),
    {
      label: t("externalApps.system"),
      onSelect: () => ids && openWith.open(ids, { kind: "system" }),
    },
    {
      label: t("externalApps.reveal"),
      onSelect: () => ids && openWith.open(ids, { kind: "reveal" }),
    },
  ];
  useEffect(() => {
    if (openWith.error) {
      notify({ tone: "error", message: openWith.error });
      openWith.clearError();
    }
  }, [openWith]);

  const more: MenuItem[] = [
    { label: t("selection.tagRemove"), onSelect: () => setRemoveTagOpen(true) },
    { label: t("selection.category"), onSelect: () => setCategoryOpen(true) },
    {
      label: t("selection.reanalyze"),
      onSelect: async () => {
        try {
          await commands.reanalyze(target);
          notify({ message: t("selection.reanalyzeStarted") });
        } catch (error) {
          notifyError(error);
        }
      },
    },
    {
      label: t("selection.organize"),
      onSelect: () => useLayout.getState().openModule({ module: "organize" }),
    },
    {
      label: t("similar.together"),
      disabled: !ids || ids.length < 2,
      onSelect: async () => {
        if (!ids) return;
        try {
          await commands.similarGroupTogether(ids);
          await qc.invalidateQueries({ queryKey: ["library"] });
          await qc.invalidateQueries({ queryKey: ["similar-group"] });
        } catch (error) {
          notifyError(error);
        }
      },
    },
    { kind: "separator" },
    ...selectItems,
    { kind: "separator" },
    ...(trashId != null && openCatalog?.id !== trashId
      ? [
          {
            label: t("selection.trash"),
            icon: <Icon.Delete size={14} />,
            shortcut: openCatalog ? undefined : "Canc",
            onSelect: () => void trash.moveToTrash(target, count),
          },
        ]
      : []),
    { label: t("selection.removeLibrary"), danger: true, onSelect: () => setRemoving(true) },
  ];

  return (
    <div role="toolbar" aria-label={t("selection.label")} className="lib-header lib-selection">
      {trash.dialog}
      <IconButton
        label={t("selection.clear")}
        shortcut="Esc"
        onClick={clear}
        icon={<Icon.Close size={16} />}
      />
      <strong data-testid="bulk-count">{t("selection.count", { count })}</strong>

      <PickerButton
        label={t("selection.tag")}
        icon={<Icon.Tags size={14} />}
        open={picker.open === "tag"}
        onOpenChange={(o) => picker.set(o ? "tag" : null)}
      >
        {(close) => (
          <SearchPicker
            label={t("selection.tagSearch")}
            placeholder={t("selection.tagSearch")}
            options={tagOptions}
            createLabel={(name) => t("selection.tagCreate", { name })}
            onPick={(name) => {
              close();
              return start({ action: "add_tag", name }, "add_tag", name);
            }}
          />
        )}
      </PickerButton>

      {openCatalog?.system && (
        <Button
          variant="primary"
          icon={<Icon.Undo size={14} />}
          onClick={() =>
            start(
              { action: "restore_from_system", catalog: openCatalog.id },
              "restore_from_system",
              catalogLabel(openCatalog, t),
            )
          }
        >
          {t("selection.restore")}
        </Button>
      )}
      {openCatalog && !openCatalog.system && (
        <Button
          variant="ghost"
          disabled={catalogAction.busy}
          onClick={() =>
            catalogAction.act(
              { target, count },
              openCatalog.id,
              openCatalog.name,
              filter.excludedFrom != null,
            )
          }
        >
          {t(filter.excludedFrom != null ? "exclusions.restore" : "exclusions.exclude")}
        </Button>
      )}
      <PickerButton label={t("selection.catalog")} icon={<Icon.Catalogs size={14} />}>
        {(close) => (
          <SearchPicker
            label={t("selection.catalogSearch")}
            placeholder={t("selection.catalogSearch")}
            options={catalogOptions}
            emptyLabel={t("selection.noCatalogs")}
            onPick={(value) => {
              close();
              return pickCatalog(value);
            }}
          />
        )}
      </PickerButton>

      <PickerButton label={t("selection.person")} icon={<Icon.People size={14} />}>
        {(close) => (
          <SearchPicker
            label={t("selection.personSearch")}
            placeholder={t("selection.personSearch")}
            options={peopleOptions}
            emptyLabel={t("selection.noPeople")}
            onPick={(value) => {
              close();
              const p = peopleOptions.find((o) => o.value === value);
              setAssigning({ id: Number(value), name: p?.label ?? "" });
            }}
          />
        )}
      </PickerButton>

      <PickerButton label={t("selection.place")} icon={<Icon.Library size={14} />}>
        {(close) => (
          <SearchPicker
            label={t("selection.placeSearch")}
            placeholder={t("selection.placeSearch")}
            options={[{ value: "", label: t("selection.placeReset") }]}
            createLabel={(name) => t("selection.placeCreate", { name })}
            onPick={(value, created) => {
              close();
              return created
                ? start({ action: "set_place", place: { name: value } }, "set_place", value)
                : start({ action: "set_place", place: null }, "reset_place");
            }}
          />
        )}
      </PickerButton>

      {ids ? (
        <Menu
          trigger={
            <button type="button" className="btn btn-ghost">
              {t("selection.openWith")}
              <Icon.ChevronDown size={14} aria-hidden="true" />
            </button>
          }
          items={openWithItems}
        />
      ) : (
        <Button
          variant="ghost"
          disabledReason={t("selection.openWithIds")}
          onClick={() => undefined}
        >
          {t("selection.openWith")}
        </Button>
      )}
      <Button
        variant="ghost"
        icon={<Icon.Export size={14} />}
        onClick={() => useLayout.getState().openModule({ module: "organize" })}
      >
        {t("selection.export")}
      </Button>

      <div className="lib-header-actions">
        <Menu
          align="end"
          trigger={
            <button type="button" className="btn btn-ghost">
              {t("common.more")}
              <Icon.More size={14} aria-hidden="true" />
            </button>
          }
          items={more}
        />
      </div>

      <Dialog open={removeTagOpen} onOpenChange={setRemoveTagOpen} title={t("selection.tagRemove")}>
        <SearchPicker
          label={t("selection.tagRemoveSearch")}
          options={tagOptions}
          onPick={(name) => {
            setRemoveTagOpen(false);
            return start({ action: "remove_tag", name }, "remove_tag", name);
          }}
        />
      </Dialog>
      <CategoryDialog
        open={categoryOpen}
        onOpenChange={setCategoryOpen}
        count={count}
        target={target}
        onApply={(group, value) =>
          start({ action: "set_label", group, value: value || null }, "set_label")
        }
      />
      <ConfirmDialog
        open={removing}
        onOpenChange={setRemoving}
        title={t("selection.removeTitle", { count })}
        message={t("selection.removeBody")}
        confirmLabel={t("selection.removeConfirm", { count })}
        count={count}
        files="untouched"
        onConfirm={() => start({ action: "remove_library" }, "remove_library")}
      />
      <ConfirmDialog
        open={assigning != null}
        onOpenChange={(o) => !o && setAssigning(null)}
        title={t("selection.assignTitle", { name: assigning?.name ?? "" })}
        message={t("selection.assignBody", { name: assigning?.name ?? "", count })}
        confirmLabel={t("selection.assignConfirm", { name: assigning?.name ?? "" })}
        danger={false}
        onConfirm={() =>
          assigning &&
          start({ action: "assign_person", person: assigning.id }, "assign_person", assigning.name)
        }
      />
    </div>
  );
}

/** Categoria della tassonomia per la selezione, con creazione di una voce nuova. */
function CategoryDialog({
  open,
  onOpenChange,
  count,
  target,
  onApply,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  count: number;
  target: Target;
  onApply: (group: string, value: string) => unknown;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const taxonomy = useTaxonomy();
  const names = useLabelName();
  const [group, setGroup] = useState("subject");
  const [value, setValue] = useState("");
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("selection.categoryTitle", { count })}
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>{t("common.cancel")}</Button>
          <Button
            variant="primary"
            disabled={!value}
            onClick={async () => {
              await onApply(group, value);
              onOpenChange(false);
            }}
          >
            {t("common.apply")}
          </Button>
        </>
      }
    >
      <FormField label={t("catalogs.field")}>
        <select
          className="field"
          value={group}
          onChange={(e) => {
            setGroup(e.target.value);
            setValue("");
          }}
        >
          {(taxonomy.data?.groups ?? []).map((g) => (
            <option key={g.id} value={g.id}>
              {names.group(g.id)}
            </option>
          ))}
        </select>
      </FormField>
      <CategoryPicker
        ariaLabel={t("bulk.value")}
        group={taxonomy.data?.groups.find((g) => g.id === group)}
        value={value}
        onChange={setValue}
        onCreate={async (label) => {
          // La voce nuova è creata e assegnata subito alla selezione.
          const r = await commands.addCategory(group, label, [], target);
          await qc.invalidateQueries({ queryKey: ["taxonomy"] });
          setValue(r.id);
          onOpenChange(false);
        }}
      />
    </Dialog>
  );
}
