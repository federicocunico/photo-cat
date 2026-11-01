//! Export atomico e organizzazione non distruttiva dei file originali.
pub mod export;
pub mod organize;
pub mod paths;
pub mod sequence;
use pc_core::AppResult;
use pc_store::{
    query::{self, LibraryFilter},
    Store,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "scope", rename_all = "snake_case")]
// Valore di breve durata (parametro di un job): la differenza di dimensione è irrilevante.
#[allow(clippy::large_enum_variant)]
pub enum Scope {
    Library,
    Catalog { id: i64 },
    Selection { ids: Vec<i64> },
    Filter { filter: LibraryFilter },
}
pub fn ids(store: &Store, scope: &Scope) -> AppResult<Vec<i64>> {
    let mut ids = match scope {
        Scope::Selection { ids } => ids.clone(),
        Scope::Library => store.read(|c| query::ids(c, &LibraryFilter::default()))?,
        Scope::Catalog { id } => store.read(|c| {
            query::ids(
                c,
                &LibraryFilter {
                    catalog_id: Some(*id),
                    ..Default::default()
                },
            )
        })?,
        Scope::Filter { filter } => store.read(|c| query::ids(c, filter))?,
    };
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}
