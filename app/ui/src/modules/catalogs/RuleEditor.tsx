import { useTranslation } from "react-i18next";
import { useLabelName, useTaxonomy } from "../library/labelName";
export type RuleNode =
  { op: "and" | "or"; rules: RuleNode[] } | { field: string; cmp: string; value: string | number };
export const newRule = (): RuleNode => ({ field: "label.subject", cmp: "eq", value: "paesaggio" });
const DEFAULT_GROUPS = ["subject", "location", "environment"];
const otherFields = [
  "tag",
  "person",
  "place",
  "format",
  "year",
  "people_count",
  "width",
  "height",
  "size",
  "name",
  "status",
];
const numeric = new Set(["year", "people_count", "width", "height", "size"]);
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
  const names = useLabelName();
  const taxonomy = useTaxonomy().data;
  // Un campo `label.<gruppo>` per ogni gruppo della tassonomia, anche quelli creati dall'utente.
  const groups = taxonomy?.groups.map((g) => g.id) ?? DEFAULT_GROUPS;
  const fields = [...groups.map((g) => `label.${g}`), ...otherFields];
  const labelGroup =
    "field" in rule && rule.field.startsWith("label.") ? rule.field.slice(6) : null;
  const categories = taxonomy?.groups.find((g) => g.id === labelGroup)?.categories;
  if ("op" in rule)
    return (
      <fieldset style={{ border: "1px solid var(--pc-border)", padding: 12, margin: "8px 0" }}>
        <legend>
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
          <div key={i} className="feature-actions">
            <RuleEditor
              rule={r}
              depth={depth + 1}
              onChange={(r) =>
                onChange({ ...rule, rules: rule.rules.map((v, j) => (i === j ? r : v)) })
              }
            />
            <button
              className="btn"
              disabled={rule.rules.length === 1}
              onClick={() => onChange({ ...rule, rules: rule.rules.filter((_, j) => j !== i) })}
            >
              {t("catalogs.removeRule")}
            </button>
          </div>
        ))}
        <button
          className="btn"
          onClick={() => onChange({ ...rule, rules: [...rule.rules, newRule()] })}
        >
          {t("catalogs.addRule")}
        </button>
        {depth < 5 && (
          <button
            className="btn"
            onClick={() =>
              onChange({ ...rule, rules: [...rule.rules, { op: "or", rules: [newRule()] }] })
            }
          >
            {t("catalogs.addGroup")}
          </button>
        )}
      </fieldset>
    );
  return (
    <div className="feature-actions">
      <select
        className="field"
        aria-label={t("catalogs.field")}
        value={rule.field}
        onChange={(e) =>
          onChange({ ...rule, field: e.target.value, value: numeric.has(e.target.value) ? 0 : "" })
        }
      >
        {fields.map((f) => (
          <option key={f} value={f}>
            {f.startsWith("label.") ? names.group(f.slice(6)) : f}
          </option>
        ))}
      </select>
      <select
        className="field"
        aria-label={t("catalogs.comparison")}
        value={rule.cmp}
        onChange={(e) => onChange({ ...rule, cmp: e.target.value })}
      >
        {[
          "eq",
          "neq",
          ...(numeric.has(rule.field) ? ["gt", "gte", "lt", "lte"] : ["contains"]),
        ].map((c) => (
          <option key={c}>{c}</option>
        ))}
      </select>
      {labelGroup && categories && rule.cmp !== "contains" ? (
        <select
          className="field"
          aria-label={t("catalogs.value")}
          value={String(rule.value)}
          onChange={(e) => onChange({ ...rule, value: e.target.value })}
        >
          <option value="">—</option>
          {categories.map((c) => (
            <option key={c.id} value={c.id}>
              {names.label(labelGroup, c.id)}
            </option>
          ))}
        </select>
      ) : (
        <input
          className="field"
          aria-label={t("catalogs.value")}
          type={numeric.has(rule.field) ? "number" : "text"}
          value={rule.value}
          onChange={(e) =>
            onChange({
              ...rule,
              value: numeric.has(rule.field) ? Number(e.target.value) : e.target.value,
            })
          }
        />
      )}
    </div>
  );
}
