//! Regole dei cataloghi smart: JSON validato e compilato in SQL parametrizzato.
//!
//! ```json
//! { "op": "and", "rules": [
//!     { "field": "label.subject", "cmp": "eq", "value": "ritratto" },
//!     { "field": "year", "cmp": "gte", "value": 2022 },
//!     { "op": "or", "rules": [ ... ] } ] }
//! ```
//! I nomi di campo sono in una lista chiusa; i valori sono sempre parametri.

use crate::{StoreError, StoreResult};
use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

const MAX_DEPTH: usize = 8;
const MAX_RULES: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Rule {
    Group {
        op: BoolOp,
        rules: Vec<Rule>,
    },
    Cond {
        field: String,
        cmp: Cmp,
        value: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cmp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    Contains,
}

impl Cmp {
    fn sql(self) -> &'static str {
        match self {
            Cmp::Eq => "=",
            Cmp::Neq => "<>",
            Cmp::Lt => "<",
            Cmp::Lte => "<=",
            Cmp::Gt => ">",
            Cmp::Gte => ">=",
            Cmp::In | Cmp::Contains => unreachable!(),
        }
    }
}

/// Frammento SQL `WHERE` (riferito agli alias `m`, `d`, `t`) con i suoi parametri.
#[derive(Debug, Default, Clone)]
pub struct SqlFragment {
    pub sql: String,
    pub args: Vec<Value>,
}

pub fn parse(json: &serde_json::Value) -> StoreResult<Rule> {
    let rule: Rule = serde_json::from_value(json.clone())
        .map_err(|e| StoreError::Invalid(format!("regola non valida: {e}")))?;
    let mut count = 0;
    validate(&rule, 0, &mut count)?;
    Ok(rule)
}

fn validate(rule: &Rule, depth: usize, count: &mut usize) -> StoreResult<()> {
    *count += 1;
    if depth > MAX_DEPTH || *count > MAX_RULES {
        return Err(StoreError::Invalid("regola troppo complessa".into()));
    }
    match rule {
        Rule::Group { rules, .. } => {
            if rules.is_empty() {
                return Err(StoreError::Invalid("gruppo di regole vuoto".into()));
            }
            rules.iter().try_for_each(|r| validate(r, depth + 1, count))
        }
        Rule::Cond { field, cmp, value } => {
            let kind = field_kind(field)?;
            check_value(kind, *cmp, value, field)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Number,
    Exists,
}

fn field_kind(field: &str) -> StoreResult<Kind> {
    if let Some(grp) = field.strip_prefix("label.") {
        return if !grp.is_empty() && grp.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            Ok(Kind::Exists)
        } else {
            Err(StoreError::Invalid(format!("gruppo non valido: {grp}")))
        };
    }
    Ok(match field {
        "person" | "tag" | "catalog" => Kind::Exists,
        "format" | "place" | "captured_at" | "status" | "text" | "name" => Kind::Text,
        "year" | "people_count" | "width" | "height" | "size" | "source" => Kind::Number,
        other => return Err(StoreError::Invalid(format!("campo sconosciuto: {other}"))),
    })
}

fn check_value(kind: Kind, cmp: Cmp, v: &serde_json::Value, field: &str) -> StoreResult<()> {
    let bad = || {
        Err(StoreError::Invalid(format!(
            "valore o confronto non valido per {field}"
        )))
    };
    let scalar_ok = |v: &serde_json::Value| match kind {
        Kind::Number => v.is_number(),
        _ => v.is_string() || (field == "catalog" && v.is_number()),
    };
    match (kind, cmp) {
        (_, Cmp::In) => match v.as_array() {
            Some(a) if !a.is_empty() && a.len() <= 500 && a.iter().all(scalar_ok) => Ok(()),
            _ => bad(),
        },
        (Kind::Exists, Cmp::Eq | Cmp::Neq) if scalar_ok(v) => Ok(()),
        (Kind::Exists, _) => bad(),
        (Kind::Text, Cmp::Contains) if v.is_string() => Ok(()),
        (Kind::Text, _) if field == "text" => bad(),
        (Kind::Number, Cmp::Contains) => bad(),
        (_, _) if scalar_ok(v) => Ok(()),
        _ => bad(),
    }
}

fn to_sql_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Number(n) if n.is_i64() => Value::Integer(n.as_i64().unwrap()),
        serde_json::Value::Number(n) => Value::Real(n.as_f64().unwrap_or_default()),
        serde_json::Value::String(s) => Value::Text(s.clone()),
        other => Value::Text(other.to_string()),
    }
}

/// Compila una regola già validata.
pub fn compile(rule: &Rule) -> StoreResult<SqlFragment> {
    let mut f = SqlFragment::default();
    compile_into(rule, &mut f)?;
    Ok(f)
}

fn compile_into(rule: &Rule, f: &mut SqlFragment) -> StoreResult<()> {
    match rule {
        Rule::Group { op, rules } => {
            let joiner = match op {
                BoolOp::And => " AND ",
                BoolOp::Or => " OR ",
            };
            f.sql.push('(');
            for (i, r) in rules.iter().enumerate() {
                if i > 0 {
                    f.sql.push_str(joiner);
                }
                compile_into(r, f)?;
            }
            f.sql.push(')');
        }
        Rule::Cond { field, cmp, value } => compile_cond(field, *cmp, value, f)?,
    }
    Ok(())
}

fn values(value: &serde_json::Value) -> Vec<Value> {
    match value.as_array() {
        Some(a) => a.iter().map(to_sql_value).collect(),
        None => vec![to_sql_value(value)],
    }
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

fn compile_cond(
    field: &str,
    cmp: Cmp,
    value: &serde_json::Value,
    f: &mut SqlFragment,
) -> StoreResult<()> {
    let vals = values(value);
    // Campi "esiste un legame" (etichetta, persona, tag, catalogo).
    let exists = |inner: &str| -> String {
        let cond = if cmp == Cmp::In {
            format!("IN ({})", placeholders(vals.len()))
        } else {
            "= ?".to_string()
        };
        let neg = if cmp == Cmp::Neq { "NOT " } else { "" };
        format!("{neg}EXISTS ({})", inner.replace("{cond}", &cond))
    };
    if let Some(grp) = field.strip_prefix("label.") {
        f.sql.push_str(&exists(
            "SELECT 1 FROM label l WHERE l.media_id = m.id AND l.grp = ? AND l.value {cond}",
        ));
        f.args.push(Value::Text(grp.to_string()));
        f.args.extend(vals);
        return Ok(());
    }
    let expr = match field {
        "person" => {
            f.sql.push_str(&exists(
                "SELECT 1 FROM face fc JOIN person p ON p.id = fc.person_id WHERE fc.media_id = m.id AND p.name {cond} COLLATE NOCASE",
            ));
            f.args.extend(vals);
            return Ok(());
        }
        "tag" => {
            f.sql.push_str(&exists(
                "SELECT 1 FROM media_tag mt JOIN tag tg ON tg.id = mt.tag_id WHERE mt.media_id = m.id AND tg.name {cond}",
            ));
            f.args.extend(vals);
            return Ok(());
        }
        "catalog" => {
            f.sql.push_str(&exists(
                "SELECT 1 FROM catalog_item ci WHERE ci.media_id = m.id AND ci.catalog_id {cond}",
            ));
            f.args.extend(vals);
            return Ok(());
        }
        "text" => {
            f.sql
                .push_str("m.id IN (SELECT rowid FROM media_fts WHERE media_fts MATCH ?)");
            f.args
                .push(Value::Text(fts_query(value.as_str().unwrap_or_default())));
            return Ok(());
        }
        "format" => "m.ext",
        "name" => "m.name",
        "status" => "m.status",
        "place" => "(SELECT lo.name FROM location lo WHERE lo.media_id = m.id)",
        "captured_at" => "d.captured_at",
        "year" => "CAST(substr(d.captured_at, 1, 4) AS INTEGER)",
        "people_count" => {
            "COALESCE((SELECT mp.count FROM media_people mp WHERE mp.media_id = m.id), 0)"
        }
        "width" => "t.width",
        "height" => "t.height",
        "size" => "m.size",
        "source" => "m.source_id",
        other => return Err(StoreError::Invalid(format!("campo sconosciuto: {other}"))),
    };
    let vals = if field == "format" {
        vals.into_iter()
            .map(|v| match v {
                Value::Text(s) => Value::Text(s.to_ascii_lowercase()),
                o => o,
            })
            .collect()
    } else {
        vals
    };
    match cmp {
        Cmp::In => {
            f.sql
                .push_str(&format!("{expr} IN ({})", placeholders(vals.len())));
            f.args.extend(vals);
        }
        Cmp::Contains => {
            f.sql.push_str(&format!("{expr} LIKE ? ESCAPE '\\'"));
            let s = value.as_str().unwrap_or_default();
            f.args.push(Value::Text(format!("%{}%", escape_like(s))));
        }
        _ => {
            f.sql.push_str(&format!("{expr} {} ?", cmp.sql()));
            f.args.extend(vals);
        }
    }
    Ok(())
}

pub fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Trasforma il testo dell'utente in una query FTS5 sicura: ogni parola è
/// racchiusa tra virgolette e usata come prefisso; le parole vanno tutte trovate.
pub fn fts_query(text: &str) -> String {
    let terms: Vec<String> = text
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\"*"))
        .collect();
    if terms.is_empty() {
        "\"\"".into()
    } else {
        terms.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_and_compiles_nested_rules() {
        let r = parse(&json!({"op": "and", "rules": [
            {"field": "label.subject", "cmp": "eq", "value": "ritratto"},
            {"field": "year", "cmp": "gte", "value": 2022},
            {"op": "or", "rules": [
                {"field": "person", "cmp": "eq", "value": "Giulia"},
                {"field": "tag", "cmp": "in", "value": ["mare", "estate"]}
            ]}
        ]}))
        .unwrap();
        let f = compile(&r).unwrap();
        assert!(f.sql.starts_with('(') && f.sql.contains(" OR "));
        assert_eq!(f.args.len(), 6);
        assert!(!f.sql.contains("ritratto") && !f.sql.contains("Giulia"));
    }

    #[test]
    fn rejects_unknown_fields_and_injection_attempts() {
        for bad in [
            json!({"field": "m.id); DROP TABLE media_item; --", "cmp": "eq", "value": 1}),
            json!({"field": "label.subject' OR 1=1", "cmp": "eq", "value": "x"}),
            json!({"field": "year", "cmp": "contains", "value": "20"}),
            json!({"field": "year", "cmp": "eq", "value": "2022"}),
            json!({"field": "tag", "cmp": "gt", "value": "x"}),
            json!({"field": "format", "cmp": "in", "value": []}),
            json!({"op": "and", "rules": []}),
            json!({"op": "xor", "rules": [{"field": "year", "cmp": "eq", "value": 1}]}),
            json!("SELECT 1"),
        ] {
            assert!(parse(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn depth_is_limited() {
        let mut r = json!({"field": "year", "cmp": "eq", "value": 2020});
        for _ in 0..12 {
            r = json!({"op": "and", "rules": [r]});
        }
        assert!(parse(&r).is_err());
    }

    #[test]
    fn fts_query_quotes_terms() {
        assert_eq!(fts_query("Siena  palio"), "\"Siena\"* \"palio\"*");
        assert_eq!(fts_query("a\" OR b"), "\"a\"* \"OR\"* \"b\"*");
    }
}
