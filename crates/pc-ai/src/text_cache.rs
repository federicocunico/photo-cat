//! Cache persistente dei prompt-ensemble, invalidata da tassonomia e pesi.
use crate::{
    clip,
    taxonomy::{Category, Taxonomy},
};
use pc_core::{AppError, AppResult, ErrorCode};
use pc_models::{pool::ModelPool, store::ModelStore, Role};
use pc_store::Store;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

const KEY: &str = "ai.text_embeddings";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub group: String,
    pub value: String,
    pub vector: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
struct Cache {
    taxonomy: Taxonomy,
    model_key: String,
    concepts: Vec<Concept>,
}

fn valid(v: &[f32]) -> bool {
    v.len() == clip::DIM
        && v.iter().all(|x| x.is_finite())
        && (v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.01
}

pub fn cached_or_compute(
    store: &Store,
    taxonomy: &Taxonomy,
    model_key: &str,
    mut encode: impl FnMut(&Category) -> AppResult<Vec<f32>>,
) -> AppResult<Vec<Concept>> {
    taxonomy.validate()?;
    let cached: Option<String> = store.read(|c| {
        Ok(
            c.query_row("SELECT value FROM settings_kv WHERE key=?1", [KEY], |r| {
                r.get(0)
            })
            .optional()?,
        )
    })?;
    if let Some(c) = cached.and_then(|s| serde_json::from_str::<Cache>(&s).ok()) {
        let expected: Vec<_> = taxonomy
            .groups
            .iter()
            .flat_map(|g| {
                g.categories
                    .iter()
                    .filter(|c| c.enabled)
                    .map(move |c| (&g.id, &c.id))
            })
            .collect();
        if c.taxonomy == *taxonomy
            && c.model_key == model_key
            && c.concepts.len() == expected.len()
            && c.concepts
                .iter()
                .zip(expected)
                .all(|(c, (g, id))| &c.group == g && &c.value == id && valid(&c.vector))
        {
            return Ok(c.concepts);
        }
    }
    let mut concepts = Vec::new();
    for g in &taxonomy.groups {
        for c in g.categories.iter().filter(|c| c.enabled) {
            let vector = encode(c)?;
            if !valid(&vector) {
                return Err(AppError::new(
                    ErrorCode::ModelUnavailable,
                    "embedding testuale non valido",
                ));
            }
            concepts.push(Concept {
                group: g.id.clone(),
                value: c.id.clone(),
                vector,
            });
        }
    }
    let cache = Cache {
        taxonomy: taxonomy.clone(),
        model_key: model_key.into(),
        concepts: concepts.clone(),
    };
    let json = serde_json::to_string(&cache)?;
    store.write(move |c| { c.execute("INSERT INTO settings_kv(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![KEY,json])?; Ok(()) })?;
    Ok(concepts)
}

pub fn from_models(
    store: &Store,
    taxonomy: &Taxonomy,
    models: &ModelStore,
    pool: &ModelPool,
) -> AppResult<Vec<Concept>> {
    let text = models
        .manifest()
        .by_role(Role::ClassificationText)
        .ok_or_else(|| AppError::invalid_input("encoder testo mancante dal manifest"))?;
    let tokenizer = models
        .manifest()
        .by_role(Role::Tokenizer)
        .ok_or_else(|| AppError::invalid_input("tokenizer mancante dal manifest"))?;
    let key = format!("{}:{}:{}", text.version, text.sha256, tokenizer.sha256);
    let mut resources = None;
    let result = cached_or_compute(store, taxonomy, &key, |category| {
        if resources.is_none() {
            let path = models.verified_path(&tokenizer.id).ok_or_else(|| {
                AppError::new(ErrorCode::ModelUnavailable, "tokenizer non verificato")
            })?;
            resources = Some((pool.get(&text.id)?, clip::ClipTokenizer::load(&path)?));
        }
        let (model, tok) = resources.as_ref().expect("risorse inizializzate");
        clip::embed_concept(&mut model.session.lock(), tok, &category.prompts)
    });
    // L'encoder testo non resta residente dopo aver rigenerato la tassonomia.
    let computed = resources.is_some();
    drop(resources);
    if computed {
        pool.unload(&text.id);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_avoids_loading_encoder_and_invalidates_on_version_or_model_change() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        let mut taxonomy = Taxonomy::default();
        let calls = std::cell::Cell::new(0);
        let encode = |_: &Category| {
            calls.set(calls.get() + 1);
            let mut v = vec![0.0; clip::DIM];
            v[0] = 1.0;
            Ok(v)
        };
        assert_eq!(
            cached_or_compute(&db, &taxonomy, "model1", encode)
                .unwrap()
                .len(),
            28
        );
        assert_eq!(calls.get(), 28);
        cached_or_compute(&db, &taxonomy, "model1", |_| {
            panic!("cache deve evitare l'encoder")
        })
        .unwrap();
        taxonomy.set_enabled("subject", "fiori", false).unwrap();
        assert_eq!(
            cached_or_compute(&db, &taxonomy, "model1", encode)
                .unwrap()
                .len(),
            27
        );
        assert_eq!(calls.get(), 55);
        cached_or_compute(&db, &taxonomy, "model2", encode).unwrap();
        assert_eq!(calls.get(), 82);
        db.close();
        let again = Store::open(&tmp.path().join("c.db"), Default::default()).unwrap();
        cached_or_compute(&again, &taxonomy, "model2", |_| panic!("cache persistente")).unwrap();
    }
}
