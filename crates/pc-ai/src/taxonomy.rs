//! Tassonomia modificabile e versionata, conservata nel catalogo locale.
use pc_core::{AppError, AppResult};
use pc_store::Store;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const KEY: &str = "ai.taxonomy";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub prompts: Vec<String>,
    /// Soglia propria della categoria; `None` = soglia del gruppo.
    #[serde(default)]
    pub threshold: Option<f32>,
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
    /// Versione delle modifiche (concorrenza ottimistica): cresce a ogni salvataggio.
    pub version: i64,
    /// Versione dell'analisi automatica, registrata nei risultati CLIP: cambia
    /// solo quando cambia la parte della tassonomia che l'analisi usa. Assente
    /// nelle tassonomie salvate prima del campo (0 = uguale a `version`).
    #[serde(default)]
    pub analysis_version: i64,
    pub groups: Vec<Group>,
}

impl Category {
    /// Voce manuale: senza descrizione per il riconoscimento automatico.
    pub fn is_manual(&self) -> bool {
        self.prompts.is_empty()
    }
    /// Voce che l'analisi automatica può assegnare.
    pub fn is_automatic(&self) -> bool {
        self.enabled && !self.is_manual()
    }
}

/// Id leggibile da un nome: minuscole senza accenti, altri caratteri → `_`.
fn slug(name: &str, allowed: impl Fn(char) -> bool) -> String {
    let folded = crate::geo::fold(name.trim());
    let mut out = String::new();
    for ch in folded.chars() {
        let ch = if allowed(ch) { ch } else { '_' };
        if ch == '_' && (out.is_empty() || out.ends_with('_')) {
            continue;
        }
        out.push(ch);
    }
    out.trim_end_matches('_').to_string()
}

/// Nome confrontato senza maiuscole, accenti e spazi superflui.
pub fn same_name(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        crate::geo::fold(s)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    norm(a) == norm(b)
}

/// Id univoco di una voce del gruppo (`barche`, `barche_2`, …).
pub fn category_id(group: &Group, name: &str) -> String {
    let base = slug(name, |c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let base = if base.is_empty() { "voce".into() } else { base };
    let taken = |id: &str| group.categories.iter().any(|c| c.id == id);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !taken(id))
        .expect("id libero")
}

/// Id univoco di un gruppo, solo `[a-z_]` come richiede la scrittura delle
/// etichette (`evento`, `evento_b`, …).
pub fn group_id(taxonomy: &Taxonomy, name: &str) -> String {
    let base = slug(name, |c| c.is_ascii_lowercase());
    let base = if base.is_empty() {
        "gruppo".into()
    } else {
        base
    };
    let taken = |id: &str| taxonomy.groups.iter().any(|g| g.id == id);
    if !taken(&base) {
        return base;
    }
    let letters = |mut n: usize| {
        let mut s = String::new();
        loop {
            s.insert(0, (b'a' + (n % 26) as u8) as char);
            if n < 26 {
                break s;
            }
            n = n / 26 - 1;
        }
    };
    (1..)
        .map(|n| format!("{base}_{}", letters(n)))
        .find(|id| !taken(id))
        .expect("id libero")
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
        threshold: None,
    }
}

impl Group {
    /// Soglia effettiva di una categoria (propria o del gruppo).
    pub fn threshold_of(&self, category: &str) -> f32 {
        self.categories
            .iter()
            .find(|c| c.id == category)
            .and_then(|c| c.threshold)
            .unwrap_or(self.threshold)
    }
}

/// Parti della tassonomia che determinano gli embedding testuali: voci
/// automatiche con i loro prompt, in ordine (senza versioni, nomi, soglie e
/// voci manuali).
pub type PromptSignature = Vec<(String, Vec<(String, Vec<String>)>)>;

/// Parti della tassonomia che determinano l'analisi automatica, senza ordine:
/// soglie dei gruppi e voci automatiche con prompt e soglia.
type AnalysisSignature = Vec<(String, u32, Vec<(String, Vec<String>, Option<u32>)>)>;

impl Taxonomy {
    pub fn prompt_signature(&self) -> PromptSignature {
        self.groups
            .iter()
            .map(|g| {
                (
                    g.id.clone(),
                    g.categories
                        .iter()
                        .filter(|c| c.is_automatic())
                        .map(|c| (c.id.clone(), c.prompts.clone()))
                        .collect(),
                )
            })
            .collect()
    }

    fn analysis_signature(&self) -> AnalysisSignature {
        let mut groups: AnalysisSignature = self
            .groups
            .iter()
            .map(|g| {
                let mut cats: Vec<_> = g
                    .categories
                    .iter()
                    .filter(|c| c.is_automatic())
                    .map(|c| {
                        let mut prompts = c.prompts.clone();
                        prompts.sort();
                        (c.id.clone(), prompts, c.threshold.map(f32::to_bits))
                    })
                    .collect();
                cats.sort();
                (g.id.clone(), g.threshold.to_bits(), cats)
            })
            // Un gruppo senza voci automatiche non entra nell'analisi.
            .filter(|(_, _, cats)| !cats.is_empty())
            .collect();
        groups.sort();
        groups
    }

    /// La modifica cambia l'analisi automatica (e richiede la rianalisi).
    pub fn changes_analysis(&self, other: &Taxonomy) -> bool {
        self.analysis_signature() != other.analysis_signature()
    }

    /// Valore effettivo di `analysis_version` (tassonomie salvate prima del campo).
    fn normalized(mut self) -> Self {
        if self.analysis_version <= 0 {
            self.analysis_version = self.version;
        }
        self
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
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
            analysis_version: 1,
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
                    || c.threshold.is_some_and(|t| !(0.0..=1.0).contains(&t))
                    || c.prompts.len() > 16
                    || c.prompts
                        .iter()
                        .any(|p| p.trim().is_empty() || p.len() > 2048)
                {
                    return Err(AppError::invalid_input("categoria o prompt non valido"));
                }
            }
            // Un gruppo solo manuale non ha voci automatiche ma deve avere voci;
            // un gruppo automatico richiede almeno una voce attiva.
            if g.categories.is_empty() {
                return Err(AppError::invalid_input(
                    "ogni gruppo richiede almeno una voce",
                ));
            }
            let automatic = g.categories.iter().any(|c| !c.is_manual());
            if automatic && !g.categories.iter().any(|c| c.enabled) {
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
            // Solo il nome mostrato: i prompt (e quindi l'analisi) non cambiano.
            c.label = label.into();
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
        Some(s) => serde_json::from_str::<Taxonomy>(&s)?.normalized(),
        None => Taxonomy::default(),
    };
    t.validate()?;
    Ok(t)
}

/// Salva una versione solo se nessun'altra modifica è stata registrata nel
/// frattempo. `analysis_version` la decide il salvataggio: cresce solo se la
/// modifica cambia l'analisi automatica. Restituisce la tassonomia salvata.
pub fn save(store: &Store, taxonomy: Taxonomy, expected_version: i64) -> AppResult<Taxonomy> {
    save_with(store, taxonomy, expected_version, |_| Ok(())).map(|(t, ())| t)
}

/// Messaggio dell'errore di concorrenza (un'altra modifica salvata nel frattempo).
pub const CONFLICT: &str = "tassonomia modificata da un'altra operazione";

/// Come [`save`], eseguendo `then` nella stessa transazione (es. le etichette
/// di una voce eliminata).
pub fn save_with<T: Send + 'static>(
    store: &Store,
    taxonomy: Taxonomy,
    expected_version: i64,
    then: impl FnOnce(&rusqlite::Connection) -> pc_store::StoreResult<T> + Send + 'static,
) -> AppResult<(Taxonomy, T)> {
    taxonomy.validate()?;
    if taxonomy.version != expected_version + 1 {
        return Err(AppError::invalid_input(
            "la nuova versione deve seguire quella corrente",
        ));
    }
    let saved = store.write(move |c| {
        let current: Option<String> = c
            .query_row("SELECT value FROM settings_kv WHERE key = ?1", [KEY], |r| r.get(0))
            .optional()?;
        let current = current
            .map(|s| serde_json::from_str::<Taxonomy>(&s))
            .transpose()?
            .map(Taxonomy::normalized)
            .unwrap_or_default();
        if current.version != expected_version {
            return Err(pc_store::StoreError::Invalid(CONFLICT.into()));
        }
        let mut next = taxonomy;
        next.analysis_version = if next.changes_analysis(&current) {
            current.analysis_version.max(next.version)
        } else {
            current.analysis_version
        };
        c.execute(
            "INSERT INTO settings_kv(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![KEY, serde_json::to_string(&next)?],
        )?;
        let extra = then(c)?;
        Ok((next, extra))
    })?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manual(id: &str, label: &str) -> Category {
        Category {
            id: id.into(),
            label: label.into(),
            enabled: true,
            prompts: vec![],
            threshold: None,
        }
    }
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
        t = save(&db, t.clone(), 1).unwrap();
        assert_eq!(t.analysis_version, 2, "voce automatica: cambia l'analisi");
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

    #[test]
    fn category_threshold_is_optional_and_validated() {
        // JSON salvato prima del campo `threshold`.
        let old = r#"{"version":3,"groups":[{"id":"environment","label":"Ambiente","threshold":0.25,
            "categories":[{"id":"stanza","label":"Stanza","enabled":true,"prompts":["a room"]}]}]}"#;
        let mut t: Taxonomy = serde_json::from_str(old).unwrap();
        t.validate().unwrap();
        assert_eq!(t.groups[0].categories[0].threshold, None);
        assert_eq!(t.groups[0].threshold_of("stanza"), 0.25);
        t.groups[0].categories[0].threshold = Some(0.6);
        assert_eq!(t.groups[0].threshold_of("stanza"), 0.6);
        t.groups[0].categories[0].threshold = Some(1.5);
        assert!(t.validate().is_err());
    }

    #[test]
    fn manual_entries_and_groups_are_valid() {
        let mut t = Taxonomy::default();
        t.add("subject", manual("matrimonio_anna", "Matrimonio Anna"))
            .unwrap();
        assert!(t
            .group("subject")
            .unwrap()
            .categories
            .last()
            .unwrap()
            .is_manual());
        // Gruppo solo manuale.
        t.groups.push(Group {
            id: "evento".into(),
            label: "Evento".into(),
            threshold: 0.25,
            categories: vec![
                manual("compleanno", "Compleanno"),
                manual("laurea", "Laurea"),
            ],
        });
        t.validate().unwrap();
        // Anche con tutte le voci manuali disattivate.
        t.groups[3]
            .categories
            .iter_mut()
            .for_each(|c| c.enabled = false);
        t.validate().unwrap();
        // Gruppo vuoto: rifiutato.
        t.groups[3].categories.clear();
        assert!(t.validate().is_err());
        // Gruppo automatico senza voci attive: rifiutato come prima.
        let mut auto = Taxonomy::default();
        auto.groups[1]
            .categories
            .iter_mut()
            .for_each(|c| c.enabled = false);
        assert!(auto.validate().is_err());
        // Prompt vuoti dentro una voce automatica: rifiutati.
        let mut bad = Taxonomy::default();
        bad.groups[0].categories[0].prompts = vec![" ".into()];
        assert!(bad.validate().is_err());
    }

    #[test]
    fn ids_from_names_are_unique_and_usable() {
        let mut t = Taxonomy::default();
        let subject = t.group("subject").unwrap().clone();
        assert_eq!(category_id(&subject, "Barche"), "barche");
        assert_eq!(category_id(&subject, "  Città d'arte! "), "citta_d_arte");
        assert_eq!(category_id(&subject, "Fiori"), "fiori_2");
        assert_eq!(category_id(&subject, "???"), "voce");
        t.add("subject", manual("barche", "Barche")).unwrap();
        let subject = t.group("subject").unwrap().clone();
        assert_eq!(category_id(&subject, "barche"), "barche_2");
        assert!(same_name("fiori", "Fiori"));
        assert!(same_name("Città", "citta"));
        assert!(same_name(" Gita  scolastica", "gita scolastica"));
        assert!(!same_name("Fiori", "Fiore"));
        // Gruppi: solo [a-z_], suffisso alfabetico.
        assert_eq!(group_id(&t, "Evento 2024"), "evento");
        assert_eq!(group_id(&t, "Ambiente"), "ambiente");
        assert_eq!(group_id(&t, "environment"), "environment_b");
        assert_eq!(group_id(&t, "Città"), "citta");
        assert!(group_id(&t, "Progetto Sud")
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_'));
    }

    #[test]
    fn analysis_version_defaults_to_version_for_saved_taxonomies() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let mut old = serde_json::to_value(Taxonomy::default()).unwrap();
        old["version"] = 7.into();
        old.as_object_mut().unwrap().remove("analysisVersion");
        db.write(move |c| {
            c.execute(
                "INSERT INTO settings_kv(key,value) VALUES (?1,?2)",
                params![KEY, old.to_string()],
            )?;
            Ok(())
        })
        .unwrap();
        let t = load(&db).unwrap();
        assert_eq!((t.version, t.analysis_version), (7, 7));
    }

    /// Ogni tipo di modifica: cambia `analysis_version` solo se cambia l'analisi.
    #[test]
    fn only_changes_relevant_to_analysis_bump_analysis_version() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("catalog.db"), Default::default()).unwrap();
        let mut current = load(&db).unwrap();
        let mut step = |what: &str, bumps: bool, edit: &dyn Fn(&mut Taxonomy)| {
            let mut next = current.clone();
            edit(&mut next);
            next.version = current.version + 1;
            let saved = save(&db, next, current.version).unwrap();
            assert_eq!(
                saved.analysis_version != current.analysis_version,
                bumps,
                "{what}"
            );
            if bumps {
                assert_eq!(saved.analysis_version, saved.version, "{what}");
            }
            current = saved;
        };
        step("voce manuale aggiunta", false, &|t| {
            t.groups[0].categories.push(manual("barche", "Barche"))
        });
        step("nome cambiato", false, &|t| {
            t.groups[0].categories[8].label = "Fiori e piante".into()
        });
        step("nome del gruppo cambiato", false, &|t| {
            t.groups[0].label = "Soggetto".into()
        });
        step("gruppo manuale aggiunto", false, &|t| {
            t.groups.push(Group {
                id: "evento".into(),
                label: "Evento".into(),
                threshold: 0.25,
                categories: vec![manual("compleanno", "Compleanno")],
            })
        });
        step("voce manuale disattivata", false, &|t| {
            t.groups[0].categories.last_mut().unwrap().enabled = false
        });
        step("ordine delle voci cambiato", false, &|t| {
            t.groups[0].categories.swap(0, 1)
        });
        step("prompt cambiato", true, &|t| {
            t.groups[0].categories[0].prompts[0] = "a close-up portrait".into()
        });
        step("voce automatica disattivata", true, &|t| {
            t.groups[1].categories[0].enabled = false
        });
        step("soglia di gruppo cambiata", true, &|t| {
            t.groups[2].threshold = 0.4
        });
        step("soglia di voce cambiata", true, &|t| {
            t.groups[2].categories[0].threshold = Some(0.6)
        });
        step("voce manuale resa automatica", true, &|t| {
            let c = t.groups[0]
                .categories
                .iter_mut()
                .find(|c| c.id == "barche")
                .unwrap();
            c.enabled = true;
            c.prompts = vec!["a photo of boats".into()];
        });
    }
}
