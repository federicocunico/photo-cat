//! Voci di catalogazione create dall'utente: "Nuova voce…" durante la
//! correzione (crea o riusa e assegna) ed eliminazione con conteggio.

use crate::taxonomy::{self, category_id, same_name, Category};
use pc_core::{AppError, AppResult};
use pc_jobs::JobManager;
use pc_store::model::MediaId;
use pc_store::{bulk, Store};
use serde::{Deserialize, Serialize};

/// Voce creata o riusata, con il job che la assegna (se richiesto).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CategoryRef {
    pub group: String,
    pub id: String,
    pub label: String,
    /// `false` se esisteva già una voce con lo stesso nome.
    pub created: bool,
    /// Job "Modifica selezione" che assegna la voce.
    pub job: Option<i64>,
}

fn is_conflict(e: &AppError) -> bool {
    e.detail.contains(taxonomy::CONFLICT) || e.message_key.contains(taxonomy::CONFLICT)
}

/// Aggiunge al gruppo una voce con questo nome, o riusa quella con lo stesso
/// nome (senza maiuscole e accenti). Senza prompt la voce è manuale. In caso
/// di modifica concorrente ricarica la tassonomia e riprova una volta.
pub fn ensure(
    store: &Store,
    group: &str,
    label: &str,
    prompts: Vec<String>,
) -> AppResult<(Category, bool)> {
    let label = label.trim();
    if label.is_empty() {
        return Err(AppError::invalid_input("il nome della voce è obbligatorio"));
    }
    let prompts: Vec<String> = prompts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut retried = false;
    loop {
        let current = taxonomy::load(store)?;
        let g = current
            .group(group)
            .ok_or_else(|| AppError::invalid_input(format!("gruppo sconosciuto: {group}")))?;
        if let Some(existing) = g.categories.iter().find(|c| same_name(&c.label, label)) {
            return Ok((existing.clone(), false));
        }
        let category = Category {
            id: category_id(g, label),
            label: label.to_string(),
            enabled: true,
            prompts: prompts.clone(),
            threshold: None,
        };
        let mut next = current.clone();
        next.groups
            .iter_mut()
            .find(|g| g.id == group)
            .expect("gruppo presente")
            .categories
            .push(category.clone());
        next.version = current.version + 1;
        match taxonomy::save(store, next, current.version) {
            Ok(_) => {
                tracing::info!(
                    group,
                    id = category.id.as_str(),
                    "voce di catalogazione aggiunta"
                );
                return Ok((category, true));
            }
            Err(e) if !retried && is_conflict(&e) => retried = true,
            Err(e) => return Err(e),
        }
    }
}

/// "Nuova voce…": crea (o riusa) la voce e, se indicati, la assegna ai file
/// come correzione manuale con il job "Modifica selezione" (annullabile:
/// l'annullamento toglie le etichette, la voce resta).
pub fn add_and_assign(
    store: &Store,
    jobs: &JobManager,
    group: &str,
    label: &str,
    prompts: Vec<String>,
    assign_to: Option<Vec<MediaId>>,
) -> AppResult<CategoryRef> {
    let (category, created) = ensure(store, group, label, prompts)?;
    let job = match assign_to.filter(|ids| !ids.is_empty()) {
        Some(ids) => Some(jobs.submit(
            pc_jobs::bulk::JOB_KIND,
            serde_json::json!({
                "ids": ids,
                "operation": { "action": "set_label", "group": group, "value": category.id },
                "confirmed": false,
            }),
        )?),
        None => None,
    };
    Ok(CategoryRef {
        group: group.to_string(),
        id: category.id,
        label: category.label,
        created,
        job,
    })
}

/// Foto a cui è assegnata la voce (chiesto prima di eliminarla).
pub fn usage(store: &Store, group: &str, id: &str) -> AppResult<i64> {
    let (g, v) = (group.to_string(), id.to_string());
    Ok(store.read(move |c| bulk::label_usage(c, &g, &v))?)
}

/// Elimina la voce dalla tassonomia e toglie la sua catalogazione dalle foto,
/// nella stessa transazione e in modo annullabile (le etichette). Restituisce
/// quante foto la perdono.
pub fn delete(store: &Store, group: &str, id: &str) -> AppResult<usize> {
    let mut retried = false;
    loop {
        let current = taxonomy::load(store)?;
        let mut next = current.clone();
        let g = next
            .groups
            .iter_mut()
            .find(|g| g.id == group)
            .ok_or_else(|| AppError::invalid_input(format!("gruppo sconosciuto: {group}")))?;
        let pos = g
            .categories
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| AppError::invalid_input(format!("voce sconosciuta: {id}")))?;
        let removed = g.categories.remove(pos);
        next.version = current.version + 1;
        let (gr, v) = (group.to_string(), id.to_string());
        let restore = bulk::RemovedCategory {
            key: taxonomy::KEY.to_string(),
            category: serde_json::to_value(&removed)?,
        };
        match taxonomy::save_with(store, next, current.version, move |c| {
            bulk::remove_label_value(c, &gr, &v, Some(restore))
        }) {
            Ok((_, ids)) => {
                tracing::info!(
                    group,
                    id,
                    photos = ids.len(),
                    "voce di catalogazione eliminata"
                );
                return Ok(ids.len());
            }
            Err(e) if !retried && is_conflict(&e) => retried = true,
            Err(e) => return Err(e),
        }
    }
}
