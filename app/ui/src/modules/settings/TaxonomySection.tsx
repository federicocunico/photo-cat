import { BusyButton } from "../../components/feedback/BusyButton";
// Impostazioni › Riconoscimento AI › Tassonomia: voci e gruppi (anche manuali, senza
// descrizione), modificati in bozza e applicati con la barra "Applica";
// eliminazione con il numero di foto coinvolte e avviso quando la modifica
// cambia l'analisi automatica.
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type Category, type Taxonomy } from "../../lib/ipc";
import { groupNameOf } from "../library/labelName";
import { categoryId, changesAnalysis, groupId, isManual } from "./taxonomyEdit";

/** Campo per il nome di una voce o di un gruppo nuovo, sul posto. */
function NameField({
  label,
  onConfirm,
  onCancel,
}: {
  label: string;
  onConfirm: (name: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [error, setError] = useState(false);
  const confirm = () => (name.trim() ? onConfirm(name.trim()) : setError(true));
  return (
    <span className="feature-actions feature-actions-tight">
      <input
        className="field"
        autoFocus
        aria-label={label}
        aria-invalid={error || undefined}
        value={name}
        onChange={(e) => {
          setName(e.target.value);
          setError(false);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") confirm();
          if (e.key === "Escape") onCancel();
        }}
      />
      <BusyButton type="button" className="btn" onClick={confirm}>
        {t("entries.confirm")}
      </BusyButton>
      <BusyButton type="button" className="btn btn-ghost" onClick={onCancel}>
        {t("entries.cancel")}
      </BusyButton>
      {error && (
        <span role="alert" className="settings-save-error">
          {t("entries.required")}
        </span>
      )}
    </span>
  );
}

type Deleting = { group: string; category: Category; count: number };

export function TaxonomySection({
  tax,
  setTax,
  saved,
  defaultThreshold,
}: {
  tax: Taxonomy;
  setTax: (t: Taxonomy) => void;
  saved: Taxonomy | undefined;
  defaultThreshold: number;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();

  const [adding, setAdding] = useState<string | null>(null);
  const [addingGroup, setAddingGroup] = useState(false);
  const [deleting, setDeleting] = useState<Deleting | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const editCategory = (gi: number, ci: number, patch: Partial<Category>) =>
    setTax({
      ...tax,
      groups: tax.groups.map((g, i) =>
        i === gi
          ? {
              ...g,
              categories: g.categories.map((c, j) => (j === ci ? { ...c, ...patch } : c)),
            }
          : g,
      ),
    });
  const run = async (f: () => Promise<void>) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await f();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const askDelete = (group: string, category: Category) =>
    run(async () => {
      const persisted = saved?.groups
        .find((g) => g.id === group)
        ?.categories.some((c) => c.id === category.id);
      if (!persisted) {
        // Voce non ancora salvata: si toglie solo dalla bozza.
        setTax({
          ...tax,
          groups: tax.groups.map((g) =>
            g.id === group
              ? { ...g, categories: g.categories.filter((c) => c.id !== category.id) }
              : g,
          ),
        });
        return;
      }
      setDeleting({ group, category, count: await commands.categoryUsage(group, category.id) });
    });
  const confirmDelete = (d: Deleting) =>
    run(async () => {
      const n = await commands.deleteCategory(d.group, d.category.id);
      setDeleting(null);
      await qc.invalidateQueries({ queryKey: ["taxonomy"] });
      await qc.invalidateQueries({ queryKey: ["facets"] });
      setNotice(t("entries.deleted", { count: n }));
    });
  const reanalysis = saved !== undefined && changesAnalysis(saved, tax);
  return (
    <>
      {tax.groups.map((g, gi) => (
        <fieldset key={g.id} data-testid={`group-${g.id}`} className="settings-taxonomy-group">
          <legend>{groupNameOf(tax, t, g.id)}</legend>
          {g.categories.map((c, ci) => (
            <div className="feature-actions" key={c.id}>
              <input
                type="checkbox"
                aria-label={`${t("settings.enabled")} ${c.label}`}
                checked={c.enabled}
                onChange={(e) => editCategory(gi, ci, { enabled: e.target.checked })}
              />
              <input
                className="field"
                aria-label={`${t("people.name")} ${c.id}`}
                value={c.label}
                onChange={(e) => editCategory(gi, ci, { label: e.target.value })}
              />
              <textarea
                className="field"
                aria-label={`${t("entries.description")} ${c.id}`}
                placeholder={t("entries.description")}
                value={c.prompts.join("\n")}
                onChange={(e) =>
                  editCategory(gi, ci, {
                    prompts: e.target.value === "" ? [] : e.target.value.split("\n"),
                  })
                }
              />
              {isManual(c) && (
                <span className="muted settings-taxonomy-manual" title={t("entries.manualHint")}>
                  {t("entries.manual")}
                </span>
              )}
              <BusyButton
                type="button"
                className="btn btn-ghost"
                disabled={busy}
                aria-label={t("entries.deleteOf", { name: c.label })}
                onClick={async () => await askDelete(g.id, c)}
              >
                {t("entries.delete")}
              </BusyButton>
            </div>
          ))}
          {deleting?.group === g.id && (
            <div role="alertdialog" className="feature-actions settings-taxonomy-confirm">
              <span>
                {deleting.count > 0
                  ? t("entries.deleteConfirm", {
                      count: deleting.count,
                      name: deleting.category.label,
                    })
                  : t("entries.deleteConfirmNone", { name: deleting.category.label })}
              </span>
              <BusyButton
                type="button"
                className="btn btn-danger"
                disabled={busy}
                onClick={async () => await confirmDelete(deleting)}
              >
                {t("entries.delete")}
              </BusyButton>
              <BusyButton type="button" className="btn btn-ghost" onClick={() => setDeleting(null)}>
                {t("entries.cancel")}
              </BusyButton>
            </div>
          )}
          {adding === g.id ? (
            <NameField
              label={t("entries.entryName", { group: g.label })}
              onCancel={() => setAdding(null)}
              onConfirm={(name) => {
                setTax({
                  ...tax,
                  groups: tax.groups.map((v, i) =>
                    i === gi
                      ? {
                          ...v,
                          categories: [
                            ...v.categories,
                            {
                              id: categoryId(v, name),
                              label: name,
                              enabled: true,
                              prompts: [],
                              threshold: null,
                            },
                          ],
                        }
                      : v,
                  ),
                });
                setAdding(null);
              }}
            />
          ) : (
            <BusyButton type="button" className="btn" onClick={() => setAdding(g.id)}>
              {t("entries.addEntry")}
            </BusyButton>
          )}
        </fieldset>
      ))}
      {addingGroup ? (
        <NameField
          label={t("entries.groupName")}
          onCancel={() => setAddingGroup(false)}
          onConfirm={(name) => {
            const id = groupId(tax, name);
            setTax({
              ...tax,
              groups: [
                ...tax.groups,
                { id, label: name, threshold: defaultThreshold, categories: [] },
              ],
            });
            setAddingGroup(false);
            setAdding(id);
          }}
        />
      ) : (
        <BusyButton type="button" className="btn" onClick={() => setAddingGroup(true)}>
          {t("entries.addGroup")}
        </BusyButton>
      )}
      {reanalysis && (
        <p role="note" className="settings-taxonomy-warn">
          {t("entries.reanalysis")}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
    </>
  );
}
