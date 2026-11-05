//! Chiavi "correnti" delle fasi di analisi (modelli installati, tassonomia, parametri
//! dei simili), nel formato che i marcatori in DB usano. Sono l'unica sorgente per
//! copertura (Panoramica AI, anteprima Rianalisi) e per il filtro `analysis` della Libreria.

use crate::state::AppState;
use pc_ai::stages::{model_key, AiResources};
use pc_models::{Entry, Role};
use pc_store::coverage::StageKeys;
use pc_store::query::{LibraryFilter, Target};

/// Voce presente su disco e non nota come guasta (stessa semantica di
/// `pc_ai::similar::resolve_model`: all'avvio lo stato è `Missing` finché non gira `check_all`).
fn installed(res: &AiResources, role: Role) -> Option<&Entry> {
    use pc_models::store::ModelState;
    let e = res.entry(role).ok()?;
    res.models.installed_path(&e.id)?;
    let broken = res.models.statuses().iter().any(|s| {
        s.entry.id == e.id
            && matches!(
                s.state,
                ModelState::Error { .. } | ModelState::SelfTestFailed { .. }
            )
    });
    (!broken).then_some(e)
}

/// Chiavi correnti di ogni fase; `None` dove il modello non è installato.
/// - classificazione: chiave del modello CLIP + versione di analisi della tassonomia;
/// - persone: chiave del rilevatore (il suffisso `|floor=` è tollerato dal confronto);
/// - volti: seconda chiave di `face_analysis::versions` (rilevatore + embedding);
/// - simili: `similar::params_key` del modello risolto.
pub(crate) fn stage_keys(state: &AppState) -> StageKeys {
    let res = &state.ai_deps.resources;
    let classification = installed(res, Role::ClassificationImage).and_then(|e| {
        let tax = pc_ai::taxonomy::load(&state.store).ok()?;
        Some((model_key(e), tax.analysis_version))
    });
    let persons = installed(res, Role::PersonDetection).map(model_key);
    let faces = (installed(res, Role::FaceDetection).is_some()
        && installed(res, Role::FaceEmbedding).is_some())
    .then(|| pc_ai::face_analysis::versions(res).ok().map(|v| v.1))
    .flatten();
    let similar = Some(pc_ai::similar::params_key(
        pc_ai::similar::resolve_model(res).as_deref(),
    ));
    StageKeys {
        classification,
        persons,
        faces,
        similar,
    }
}

/// Riempie le chiavi nel filtro solo se serve (filtro `analysis` attivo): evita di
/// leggere la tassonomia a ogni pagina della griglia.
pub(crate) fn fill_filter(state: &AppState, filter: &mut LibraryFilter) {
    filter.stage_keys = filter.analysis.map(|_| stage_keys(state));
}

/// Come [`fill_filter`] per il filtro di una selezione "Seleziona tutto". Le chiavi
/// viaggiano nel `Target` serializzato, quindi restano valide anche nei job.
pub(crate) fn fill_target(state: &AppState, target: &mut Target) {
    if let Target::Filter { filter, .. } = target {
        fill_filter(state, filter);
    }
}
