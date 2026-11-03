// Editor delle regole dei cataloghi smart: ogni condizione si legge come "campo, confronto,
// valore" nella lingua dell'interfaccia. Le opzioni di campo e confronto hanno per valore la
// posizione nell'elenco, così gli id interni ("people_count", "gte"…) non arrivano nel DOM.
import { useId } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button, IconButton } from "../../components/ui";
import { Icon } from "../../components/icons";
import { commands, isTauri, type Facets } from "../../lib/ipc";
import { emptyFilter } from "../../store/library";
import { useLabelName, useTaxonomy } from "../library/labelName";
import { useTagOverview } from "../tags/data";
import {
  MB,
  OTHER_FIELDS,
  comparisons,
  fieldName,
  isNumeric,
  kindOf,
  newRule,
  withField,
  type Cond,
  type RuleNode,
} from "./rules";

const DEFAULT_GROUPS = ["subject", "location", "environment"];

/** Valori già presenti nella libreria, suggeriti mentre si scrive. */
function useSuggestions(): Record<string, string[]> {
  const facets = useQuery({
    queryKey: ["catalog-rule-values"],
    queryFn: () =>
      commands.libraryFacets(emptyFilter(), null).then((f): Facets | null => f ?? null),
    enabled: isTauri(),
    staleTime: 60_000,
  }).data;
  const tags = useTagOverview().data?.tags.map((x) => x.name);
  const list = (l: { values: Array<{ value: string }> } | undefined) =>
    l?.values.map((v) => v.value) ?? [];
  return {
    tag: tags ?? list(facets?.tags),
    person: list(facets?.people),
    place: list(facets?.places),
    format: list(facets?.formats).map((v) => v.toUpperCase()),
  };
}

export function RuleEditor({
  rule,
  onChange,
  depth = 0,
}: {
  rule: RuleNode;
  onChange: (r: RuleNode) => void;
  depth?: number;
}) {
  const { t } = useTranslation();
  if (!("op" in rule)) return <Condition rule={rule} onChange={onChange} />;
  const set = (rules: RuleNode[]) => onChange({ ...rule, rules });
  return (
    <fieldset className="cat-rule-group">
      <legend className="cat-rule-legend">
        <span>{t("catalogs.matchIntro")}</span>
        <select
          className="field"
          aria-label={t("catalogs.operator")}
          value={rule.op}
          onChange={(e) => onChange({ ...rule, op: e.target.value as "and" | "or" })}
        >
          <option value="and">{t("catalogs.and")}</option>
          <option value="or">{t("catalogs.or")}</option>
        </select>
      </legend>
      {rule.rules.map((r, i) => (
        <div key={i} className="cat-rule-row">
          <RuleEditor
            rule={r}
            depth={depth + 1}
            onChange={(r) => set(rule.rules.map((v, j) => (i === j ? r : v)))}
          />
          <IconButton
            label={t("catalogs.removeRule")}
            icon={<Icon.Delete size={16} />}
            disabled={rule.rules.length === 1}
            onClick={() => set(rule.rules.filter((_, j) => j !== i))}
          />
        </div>
      ))}
      <div className="cat-rule-actions">
        <Button
          variant="ghost"
          icon={<Icon.Plus size={16} />}
          onClick={() => set([...rule.rules, newRule()])}
        >
          {t("catalogs.addRule")}
        </Button>
        {depth < 5 && (
          <Button
            variant="ghost"
            icon={<Icon.Plus size={16} />}
            onClick={() => set([...rule.rules, { op: "or", rules: [newRule()] }])}
          >
            {t("catalogs.addGroup")}
          </Button>
        )}
      </div>
    </fieldset>
  );
}

function Condition({ rule, onChange }: { rule: Cond; onChange: (r: RuleNode) => void }) {
  const { t } = useTranslation();
  const names = useLabelName();
  const taxonomy = useTaxonomy().data;
  const suggestions = useSuggestions();
  const listId = useId();
  // Un campo `label.<gruppo>` per ogni gruppo della tassonomia, anche quelli creati dall'utente.
  const groups = taxonomy?.groups.map((g) => g.id) ?? DEFAULT_GROUPS;
  const fields = [...groups.map((g) => `label.${g}`), ...OTHER_FIELDS];
  // Un campo sconosciuto (regola scritta da una versione diversa) resta selezionabile.
  if (!fields.includes(rule.field)) fields.push(rule.field);
  const kind = kindOf(rule.field);
  const known: string[] = comparisons(null);
  const cmps: string[] = comparisons(kind);
  if (!cmps.includes(rule.cmp)) cmps.push(rule.cmp);
  const labelGroup = kind === "label" ? rule.field.slice(6) : null;
  const categories = taxonomy?.groups.find((g) => g.id === labelGroup)?.categories;
  const set = (value: Cond["value"]) => onChange({ ...rule, value });

  let value;
  if (labelGroup && categories && rule.cmp !== "contains") {
    value = (
      <select
        className="field"
        aria-label={t("catalogs.value")}
        value={String(rule.value)}
        onChange={(e) => set(e.target.value)}
      >
        <option value="">{t("catalogs.choose")}</option>
        {categories.map((c) => (
          <option key={c.id} value={c.id}>
            {names.label(labelGroup, c.id)}
          </option>
        ))}
      </select>
    );
  } else if (kind === "status") {
    value = (
      <select
        className="field"
        aria-label={t("catalogs.value")}
        value={String(rule.value)}
        onChange={(e) => set(e.target.value)}
      >
        <option value="ok">{t("catalogs.status.ok")}</option>
        <option value="missing">{t("catalogs.status.missing")}</option>
      </select>
    );
  } else if (isNumeric(kind) && !Array.isArray(rule.value)) {
    const n = Number(rule.value) || 0;
    const unit =
      kind === "count"
        ? t("catalogs.units.people", { count: n })
        : kind === "pixels"
          ? t("catalogs.units.px")
          : kind === "size"
            ? t("catalogs.units.mb")
            : null;
    value = (
      <span className="cat-rule-number">
        <input
          className="field"
          aria-label={t("catalogs.value")}
          type="number"
          min={kind === "year" ? 1900 : 0}
          max={kind === "year" ? 2100 : undefined}
          step={1}
          value={kind === "size" ? Math.round(n / MB) : n}
          onChange={(e) => {
            const v = Number(e.target.value) || 0;
            set(kind === "size" ? v * MB : v);
          }}
        />
        {unit && <span className="muted">{unit}</span>}
      </span>
    );
  } else {
    const options = (kind && suggestions[kind]) || [];
    value = (
      <>
        <input
          className="field"
          aria-label={t("catalogs.value")}
          list={options.length ? listId : undefined}
          value={
            Array.isArray(rule.value)
              ? rule.value.join(", ")
              : kind === "format"
                ? String(rule.value).toUpperCase()
                : String(rule.value)
          }
          onChange={(e) =>
            set(
              Array.isArray(rule.value)
                ? e.target.value.split(",").map((s) => s.trim())
                : kind === "format"
                  ? e.target.value.toLowerCase()
                  : e.target.value,
            )
          }
        />
        {options.length > 0 && (
          <datalist id={listId}>
            {options.map((o) => (
              <option key={o} value={o} />
            ))}
          </datalist>
        )}
      </>
    );
  }

  return (
    <div className="cat-rule-cond">
      <select
        className="field"
        aria-label={t("catalogs.field")}
        value={String(fields.indexOf(rule.field))}
        onChange={(e) => onChange(withField(rule, fields[Number(e.target.value)]))}
      >
        {fields.map((f, i) => (
          <option key={f} value={i}>
            {fieldName(t, names, f)}
          </option>
        ))}
      </select>
      <select
        className="field"
        aria-label={t("catalogs.comparison")}
        value={String(cmps.indexOf(rule.cmp))}
        onChange={(e) => onChange({ ...rule, cmp: cmps[Number(e.target.value)] })}
      >
        {cmps.map((c, i) => (
          <option key={c} value={i}>
            {known.includes(c) ? t(`catalogs.cmp.${c}`) : "—"}
          </option>
        ))}
      </select>
      {value}
    </div>
  );
}
