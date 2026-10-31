//! Tassonomia modificabile e versionata, conservata nel catalogo locale.
use pc_core::{AppError, AppResult};
use pc_store::Store;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const KEY: &str = "ai.taxonomy";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub prompts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub label: String,
    pub threshold: f32,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Taxonomy {
    pub version: i64,
    pub groups: Vec<Group>,
}

fn category(id: &str, label: &str, english: &str) -> Category {
    Category {
        id: id.into(),
        label: label.into(),
        enabled: true,
        prompts: vec![
            format!("a photo of {english}"),
            format!("a photograph showing {english}"),
        ],
    }
}

impl Default for Taxonomy {
    fn default() -> Self {
        let group = |id: &str, label: &str, entries: &[(&str, &str, &str)]| Group {
            id: id.into(),
            label: label.into(),
            threshold: 0.25,
            categories: entries
                .iter()
                .map(|&(id, name, prompt)| category(id, name, prompt))
                .collect(),
        };
        Self {
            version: 1,
            groups: vec![
                group(
                    "subject",
                    "Tipo soggetto",
                    &[
                        ("ritratto", "Ritratto", "a portrait of a person"),
                        ("gruppo", "Gruppo", "a group of people"),
                        ("natura", "Natura", "nature"),
                        ("paesaggio", "Paesaggio", "a landscape"),
                        ("urbano", "Ambiente urbano", "an urban environment"),
                        ("monumento", "Monumento", "a monument"),
                        ("opera_arte", "Opera d'arte", "a work of art"),
                        ("animale", "Animale", "an animal"),
                        ("fiori", "Fiori", "flowers"),
                        ("alberi", "Alberi", "trees"),
                        ("cibo", "Cibo", "food"),
                        ("oggetto", "Oggetto", "an object"),
                    ],
                ),
                group(
                    "location",
                    "Interno / Esterno",
                    &[
                        ("interno", "Interno", "an indoor scene"),
                        ("esterno_urbano", "Esterno urbano", "an outdoor urban scene"),
                        (
                            "esterno_natura",
                            "Esterno natura",
                            "an outdoor natural landscape",
                        ),
                    ],
                ),
                group(
                    "environment",
                    "Ambiente",
                    &[
                        ("stanza", "Stanza", "a room"),
                        ("teatro", "Teatro", "a theater"),
                        ("palazzetto", "Palazzetto", "an indoor sports arena"),
                        ("chiesa", "Chiesa", "a church interior"),
                        ("museo", "Museo", "a museum"),
                        ("via", "Via", "a street"),
                        ("piazza", "Piazza", "a city square"),
                        ("mare", "Mare", "the sea"),
                        ("montagna", "Montagna", "a snowy mountain landscape"),
                        ("lago", "Lago", "a lake"),
                        ("fiume", "Fiume", "a river"),
                        ("bosco", "Bosco", "a forest"),
                        ("campagna", "Campagna", "the countryside"),
                    ],
                ),
            ],
        }
    }
}

impl Taxonomy {
    pub fn validate(&self) -> AppResult<()> {
        let mut groups = HashSet::new();
        if self.version < 1 || self.groups.is_empty() || self.groups.len() > 32 {
            return Err(AppError::invalid_input(
                "tassonomia senza versione o gruppi validi",
            ));
        }
        for g in &self.groups {
            let mut ids = HashSet::new();
            if g.id.trim().is_empty()
                || g.label.trim().is_empty()
                || !groups.insert(&g.id)
                || !(0.0..=1.0).contains(&g.threshold)
                || g.categories.len() > 1000
            {
                return Err(AppError::invalid_input(
                    "gruppo della tassonomia non valido",
                ));
            }
            for c in &g.categories {
                if c.id.trim().is_empty()
                    || c.label.trim().is_empty()
                    || !ids.insert(&c.id)
                    || c.prompts.is_empty()
                    || c.prompts.len() > 16
                    || c.prompts
                        .iter()
                        .any(|p| p.trim().is_empty() || p.len() > 2048)
                {
                    return Err(AppError::invalid_input("categoria o prompt non valido"));
                }
            }
            if !g.categories.iter().any(|c| c.enabled) {
                return Err(AppError::invalid_input(
                    "ogni gruppo richiede una categoria attiva",
                ));
            }
        }
        Ok(())
    }

    fn edit(&mut self, group: &str, f: impl FnOnce(&mut Group) -> AppResult<()>) -> AppResult<()> {
        let mut next = self.clone();
        let g = next
            .groups
            .iter_mut()
            .find(|g| g.id == group)
            .ok_or_else(|| AppError::invalid_input("gruppo sconosciuto"))?;
        f(g)?;
        next.version = self
            .version
            .checked_add(1)
            .ok_or_else(|| AppError::invalid_input("versione tassonomia esaurita"))?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn add(&mut self, group: &str, value: Category) -> AppResult<()> {
        self.edit(group, |g| {
            g.categories.push(value);
            Ok(())
        })
    }

    pub fn rename(&mut self, group: &str, id: &str, label: &str) -> AppResult<()> {
        self.edit(group, |g| {
            let c = g
                .categories
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or_else(|| AppError::invalid_input("categoria sconosciuta"))?;
            c.label = label.into();
            c.prompts = category(id, label, label).prompts;
            Ok(())
        })
    }

    pub fn set_enabled(&mut self, group: &str, id: &str, enabled: bool) -> AppResult<()> {
        self.edit(group, |g| {
            g.categories
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or_else(|| AppError::invalid_input("categoria sconosciuta"))?
                .enabled = enabled;
            Ok(())
        })
    }
}

pub fn load(store: &Store) -> AppResult<Taxonomy> {
    let json: Option<String> = store.read(|c| {
        Ok(
            c.query_row("SELECT value FROM settings_kv WHERE key = ?1", [KEY], |r| {
                r.get(0)
            })
            .optional()?,
        )
    })?;
    let t = match json {
        Some(s) => serde_json::from_str(&s)?,
        None => Taxonomy::default(),
    };
    t.validate()?;
    Ok(t)
}

/// Salva una versione solo se nessun'altra modifica è stata registrata nel frattempo.
pub fn save(store: &Store, taxonomy: Taxonomy, expected_version: i64) -> AppResult<()> {
    taxonomy.validate()?;
    if taxonomy.version != expected_version + 1 {
        return Err(AppError::invalid_input(
            "la nuova versione deve seguire quella corrente",
        ));
    }
    store.write(move |c| {
        let current: Option<String> = c.query_row("SELECT value FROM settings_kv WHERE key = ?1", [KEY], |r| r.get(0)).optional()?;
        let version = current.map(|s| serde_json::from_str::<Taxonomy>(&s)).transpose()?.unwrap_or_default().version;
        if version != expected_version { return Err(pc_store::StoreError::Invalid("tassonomia modificata da un'altra operazione".into())); }
        c.execute("INSERT INTO settings_kv(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![KEY, serde_json::to_string(&taxonomy)?])?;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_are_versioned_validated_and_persisted() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let mut t = load(&db).unwrap();
        assert_eq!(
            t.groups.iter().map(|g| g.categories.len()).sum::<usize>(),
            28
        );
        t.add("subject", category("barche", "Barche", "boats"))
            .unwrap();
        assert_eq!(t.version, 2);
        save(&db, t.clone(), 1).unwrap();
        assert_eq!(load(&db).unwrap(), t);
        assert!(save(&db, t.clone(), 1).is_err());
        t.set_enabled("subject", "barche", false).unwrap();
        t.rename("subject", "ritratto", "Ritratti").unwrap();
        assert_eq!(t.version, 4);
        let prior = t.clone();
        assert!(t
            .add("subject", category("ritratto", "Duplicato", "portrait"))
            .is_err());
        assert_eq!(t, prior);
        assert!(t.rename("subject", "ritratto", "").is_err());
        assert_eq!(t, prior);
    }
}
